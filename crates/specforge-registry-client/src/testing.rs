//! A package registry in memory, behind the `RegistryClient` seam, and the contract every adapter of the
//! seam keeps (ADR 0044).

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use sha2::{Digest, Sha256};
use specforge_protocol_types::package::Version;
use specforge_protocol_types::{ExtensionDeclaration, PackageName};
use specforge_registry_wire::{DEFAULT_SEARCH_LIMIT, PackageMetadata, SearchHit, path};

use crate::registry_client::{RegistryClient, RegistryError};
use crate::registry_config::{AuthMethod, RegistryConfig, RegistryCredential};
use crate::{HttpRegistryClient, PackageSignature, SigningKey, TrustCheck};

/// One call a [`MemoryClient`] answered (or failed), in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Call {
    pub kind: CallKind,
    /// The alias of the registry asked; `None` for a download.
    pub registry: Option<String>,
    /// `name`, `name@version`, the query, or the download URL.
    pub subject: String,
    /// What a publish or an authenticate sent.
    pub credential: Option<RegistryCredential>,
    /// A publish's signature object.
    pub signature: Option<String>,
    /// A publish's manifest JSON.
    pub manifest: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallKind {
    Versions,
    Metadata,
    Download,
    Search,
    Publish,
    Authenticate,
}

struct Failure {
    kind: CallKind,
    /// The URL of the registry it applies to; any when `None`.
    registry: Option<String>,
    error: RegistryError,
}

/// One version a registry publishes: asked for by `name` and `version`, answered with `metadata`.
struct Stored {
    name: String,
    version: String,
    metadata: PackageMetadata,
}

#[derive(Default)]
struct State {
    /// Per registry URL, the versions it publishes, in store order.
    registries: HashMap<String, Vec<Stored>>,
    /// The binary of each stored version, by the URL it downloads from.
    binaries: HashMap<String, Vec<u8>>,
    tokens: HashSet<String>,
    failures: Vec<Failure>,
    calls: Vec<Call>,
}

/// A package registry held in memory: the second adapter of [`RegistryClient`], beside
/// [`HttpRegistryClient`]. For every call the contract exercises it answers as the registry server does.
/// It checks nothing a test [`MemoryClient::store`]s, so a test can serve any reply, honest or not, and run
/// the fetch policy without sockets. Clones share one store.
#[derive(Clone, Default)]
pub struct MemoryClient {
    state: Arc<Mutex<State>>,
}

fn key_of(registry: &RegistryConfig) -> String {
    registry.url.trim_end_matches('/').to_string()
}

/// Where a stored version's binary downloads from unless a test says otherwise.
fn download_url(registry: &RegistryConfig, name: &str, version: &str) -> String {
    format!("memory://{}/{name}/{version}", registry.alias)
}

impl MemoryClient {
    /// A registry holding nothing and accepting no token.
    pub fn new() -> Self {
        Self::default()
    }

    /// Accept `token` as a bearer credential for publish and authenticate, as a server accepts the
    /// tokens it issued.
    pub fn accepting(self, token: &str) -> Self {
        self.state.lock().unwrap().tokens.insert(token.to_string());
        self
    }

    /// Store `metadata` and `wasm` in `registry` as one published version, unchecked. An empty
    /// `metadata.wasm_url` becomes `memory://{alias}/{name}/{version}`; any other is kept, so a download
    /// can be made to miss.
    pub fn store(&self, registry: &RegistryConfig, metadata: PackageMetadata, wasm: Vec<u8>) {
        let (name, version) = (metadata.name.clone(), metadata.version.clone());
        self.store_as(registry, &name, &version, metadata, wasm);
    }

    /// [`MemoryClient::store`] `metadata` as the answer for `name@version`, whatever package and version
    /// it describes: how a test serves a reply for another package than the one asked for.
    pub fn store_as(
        &self,
        registry: &RegistryConfig,
        name: &str,
        version: &str,
        mut metadata: PackageMetadata,
        wasm: Vec<u8>,
    ) {
        let url = download_url(registry, name, version);
        if metadata.wasm_url.is_empty() {
            metadata.wasm_url = url.clone();
        }
        let mut state = self.state.lock().unwrap();
        state.binaries.insert(url, wasm);
        state
            .registries
            .entry(key_of(registry))
            .or_default()
            .push(Stored {
                name: name.to_string(),
                version: version.to_string(),
                metadata,
            });
    }

    /// Fail the next `kind` call on `registry` (any registry when `None`) with `error`. Queued failures
    /// are consumed in order.
    pub fn fail_next(
        &self,
        kind: CallKind,
        registry: Option<&RegistryConfig>,
        error: RegistryError,
    ) {
        self.state.lock().unwrap().failures.push(Failure {
            kind,
            registry: registry.map(key_of),
            error,
        });
    }

    /// Every call answered so far, oldest first.
    pub fn calls(&self) -> Vec<Call> {
        self.state.lock().unwrap().calls.clone()
    }

    /// Record `call`, then fail it when a queued failure matches.
    fn begin(&self, call: Call, registry: Option<&RegistryConfig>) -> Result<(), RegistryError> {
        let mut state = self.state.lock().unwrap();
        let kind = call.kind;
        state.calls.push(call);
        let url = registry.map(key_of);
        let at = state
            .failures
            .iter()
            .position(|f| f.kind == kind && (f.registry.is_none() || f.registry == url));
        match at {
            Some(at) => Err(state.failures.remove(at).error),
            None => Ok(()),
        }
    }

    fn call(kind: CallKind, registry: Option<&RegistryConfig>, subject: String) -> Call {
        Call {
            kind,
            registry: registry.map(|r| r.alias.clone()),
            subject,
            credential: None,
            signature: None,
            manifest: None,
        }
    }

    /// Whether `credential` is a token this registry accepts.
    fn accepts(&self, credential: Option<&RegistryCredential>) -> Result<(), RegistryError> {
        let unauthorized = |guidance: &str| RegistryError::Unauthorized {
            guidance: guidance.to_string(),
        };
        let Some(credential) = credential else {
            return Err(unauthorized("missing Authorization header"));
        };
        let token = match &credential.auth_method {
            AuthMethod::Bearer(token) => token.clone(),
            other => HttpRegistryClient::resolve_token(&RegistryCredential {
                alias: credential.alias.clone(),
                auth_method: other.clone(),
            })?,
        };
        if self.state.lock().unwrap().tokens.contains(&token) {
            Ok(())
        } else {
            Err(unauthorized("invalid or revoked token"))
        }
    }
}

impl RegistryClient for MemoryClient {
    fn versions(
        &self,
        name: &PackageName,
        registry: &RegistryConfig,
    ) -> Result<Vec<String>, RegistryError> {
        self.begin(
            Self::call(CallKind::Versions, Some(registry), name.to_string()),
            Some(registry),
        )?;
        let state = self.state.lock().unwrap();
        let versions: Vec<String> = state
            .registries
            .get(&key_of(registry))
            .into_iter()
            .flatten()
            .filter(|m| m.name == name.as_str())
            .map(|m| m.version.clone())
            .collect();
        if versions.is_empty() {
            return Err(RegistryError::NotFound {
                specifier: name.to_string(),
            });
        }
        Ok(versions)
    }

    fn metadata(
        &self,
        name: &PackageName,
        version: &Version,
        registry: &RegistryConfig,
    ) -> Result<PackageMetadata, RegistryError> {
        let subject = format!("{name}@{version}");
        self.begin(
            Self::call(CallKind::Metadata, Some(registry), subject.clone()),
            Some(registry),
        )?;
        let state = self.state.lock().unwrap();
        state
            .registries
            .get(&key_of(registry))
            .into_iter()
            .flatten()
            .find(|m| m.name == name.as_str() && m.version == version.to_string())
            .map(|m| m.metadata.clone())
            .ok_or(RegistryError::NotFound { specifier: subject })
    }

    fn download(&self, wasm_url: &str) -> Result<Vec<u8>, RegistryError> {
        self.begin(
            Self::call(CallKind::Download, None, wasm_url.to_string()),
            None,
        )?;
        let state = self.state.lock().unwrap();
        state
            .binaries
            .get(wasm_url)
            .cloned()
            .ok_or(RegistryError::NotFound {
                specifier: wasm_url.to_string(),
            })
    }

    fn search(
        &self,
        query: &str,
        registry: &RegistryConfig,
    ) -> Result<Vec<SearchHit>, RegistryError> {
        self.begin(
            Self::call(CallKind::Search, Some(registry), query.to_string()),
            Some(registry),
        )?;
        let needle = query.to_ascii_lowercase();
        let matches = |m: &Stored| {
            m.name.to_ascii_lowercase().contains(&needle)
                || m.metadata
                    .description
                    .to_ascii_lowercase()
                    .contains(&needle)
                || m.metadata
                    .keywords
                    .iter()
                    .any(|k| k.to_ascii_lowercase().contains(&needle))
        };
        let state = self.state.lock().unwrap();
        // The latest version of each matching package, by SemVer, then by name.
        let mut latest: HashMap<&str, &Stored> = HashMap::new();
        for stored in state
            .registries
            .get(&key_of(registry))
            .into_iter()
            .flatten()
        {
            if !matches(stored) {
                continue;
            }
            let newer = match latest.get(stored.name.as_str()) {
                Some(current) => {
                    Version::parse(&stored.version).ok() > Version::parse(&current.version).ok()
                }
                None => true,
            };
            if newer {
                latest.insert(&stored.name, stored);
            }
        }
        let mut hits: Vec<SearchHit> = latest
            .into_values()
            .map(|m| SearchHit {
                name: m.name.clone(),
                version: m.version.clone(),
                description: m.metadata.description.clone(),
            })
            .collect();
        hits.sort_by(|a, b| a.name.cmp(&b.name));
        hits.truncate(DEFAULT_SEARCH_LIMIT as usize);
        Ok(hits)
    }

    fn publish(
        &self,
        package: &[u8],
        declaration: &ExtensionDeclaration,
        manifest_json: &str,
        signature: Option<&str>,
        registry: &RegistryConfig,
        credential: Option<&RegistryCredential>,
    ) -> Result<String, RegistryError> {
        let name = declaration
            .package_name()
            .map_err(|why| RegistryError::InvalidPackage {
                message: why.to_string(),
            })?;
        let version =
            Version::parse(declaration.version()).map_err(|why| RegistryError::InvalidPackage {
                message: format!("'{}' is not a SemVer version: {why}", declaration.version()),
            })?;
        self.begin(
            Call {
                credential: credential.cloned(),
                signature: signature.map(str::to_string),
                manifest: Some(manifest_json.to_string()),
                ..Self::call(
                    CallKind::Publish,
                    Some(registry),
                    format!("{name}@{version}"),
                )
            },
            Some(registry),
        )?;
        self.accepts(credential)?;
        let held = self
            .state
            .lock()
            .unwrap()
            .registries
            .get(&key_of(registry))
            .into_iter()
            .flatten()
            .any(|m| m.name == name.as_str() && m.version == version.to_string());
        if held {
            return Err(RegistryError::DuplicateVersion {
                name: declaration.name().to_string(),
                version: declaration.version().to_string(),
            });
        }
        let mut metadata = package_metadata(
            declaration.name(),
            &version.to_string(),
            package,
            manifest_json,
            None,
        );
        if let Some(signature) = signature {
            metadata.key_id = serde_json::from_str::<PackageSignature>(signature)
                .map(|s| s.key_id)
                .unwrap_or_default();
            metadata.signature = signature.to_string();
        }
        self.store(registry, metadata, package.to_vec());
        Ok(format!(
            "{}{}",
            key_of(registry),
            path::version(&name, &version)
        ))
    }

    fn authenticate(
        &self,
        registry: &RegistryConfig,
        credential: &RegistryCredential,
    ) -> Result<Option<String>, RegistryError> {
        self.begin(
            Call {
                credential: Some(credential.clone()),
                ..Self::call(CallKind::Authenticate, Some(registry), registry.url.clone())
            },
            Some(registry),
        )?;
        self.accepts(Some(credential))?;
        Ok(None)
    }
}

fn sha256_hex(data: &[u8]) -> String {
    hex::encode(Sha256::digest(data))
}

fn package_metadata(
    name: &str,
    version: &str,
    wasm: &[u8],
    manifest: &str,
    key: Option<&SigningKey>,
) -> PackageMetadata {
    let sha256 = sha256_hex(wasm);
    let (description, keywords) = serde_json::from_str::<ExtensionDeclaration>(manifest)
        .map(|d| {
            (
                d.handshake.description.clone().unwrap_or_default(),
                d.handshake.keywords.clone(),
            )
        })
        .unwrap_or_default();
    let (signature, key_id) = match key {
        Some(key) => {
            let signature = key.sign_package(
                name,
                version,
                &sha256,
                &sha256_hex(manifest.as_bytes()),
                &chrono::Utc::now().to_rfc3339(),
            );
            (
                serde_json::to_string(&signature).expect("a signature serializes"),
                key.key_id(),
            )
        }
        None => (String::new(), String::new()),
    };
    PackageMetadata {
        name: name.to_string(),
        version: version.to_string(),
        sha256,
        size_bytes: wasm.len() as u64,
        description,
        keywords,
        wasm_url: String::new(),
        signature,
        key_id,
        manifest: manifest.to_string(),
        ..Default::default()
    }
}

/// What a registry stores for `name@version` built from `wasm` and `manifest`: the SHA-256 and size of
/// `wasm`, the description and keywords of the manifest's handshake (empty when it is not a declaration),
/// and, when `key` is given, the publisher signature over them and its key id. `wasm_url` is empty.
pub fn package(
    name: &str,
    version: &str,
    wasm: &[u8],
    manifest: &str,
    key: Option<&SigningKey>,
) -> PackageMetadata {
    package_metadata(name, version, wasm, manifest, key)
}

/// The declaration the contract publishes: `@contract/tool` at `version`, described "Contract tool".
fn contract_declaration(version: &str) -> ExtensionDeclaration {
    serde_json::from_value(serde_json::json!({
        "handshake": {
            "protocol_version": "1.0.0",
            "name": "@contract/tool",
            "version": version,
            "description": "Contract tool",
            "keywords": ["contract"],
            "contribution_flags": {},
            "peer_dependencies": [],
            "sandbox_policy": null
        }
    }))
    .expect("the contract declaration is a declaration")
}

/// The client contract (ADR 0044): what every `RegistryClient` adapter does against a registry that holds
/// nothing yet. `credential` must be one `registry` accepts for publishing. Panics naming the clause that
/// failed.
pub fn assert_client_contract(
    client: &dyn RegistryClient,
    registry: &RegistryConfig,
    credential: &RegistryCredential,
) {
    let tool = PackageName::parse("@contract/tool").unwrap();
    let key = SigningKey::generate();
    let base = registry.url.trim_end_matches('/');

    // K1
    let error = client.versions(&tool, registry).unwrap_err();
    assert!(
        matches!(error, RegistryError::NotFound { .. }),
        "K1: versions before any publish is NotFound, not {error:?}"
    );

    // K2
    let mut declarations = Vec::new();
    for version in ["1.0.0", "1.1.0"] {
        let declaration = contract_declaration(version);
        let wasm = format!("\0asm contract tool {version}").into_bytes();
        let url = crate::publish_to_registry(
            &wasm,
            &declaration,
            registry,
            Some(credential),
            client,
            Some(&key),
        )
        .unwrap_or_else(|d| panic!("K2: publishing {version} fails: {d:?}"));
        let expected = format!(
            "{base}{}",
            path::version(&tool, &Version::parse(version).unwrap())
        );
        assert_eq!(url, expected, "K2: publish answers the version's URL");
        declarations.push((declaration, wasm));
    }

    // K3
    let mut versions = client.versions(&tool, registry).expect("K3: versions");
    versions.sort();
    assert_eq!(versions, ["1.0.0", "1.1.0"], "K3: the published versions");

    // K4
    let (declaration, wasm) = &declarations[1];
    let metadata = client
        .metadata(&tool, &Version::new(1, 1, 0), registry)
        .expect("K4: metadata of 1.1.0");
    assert_eq!(metadata.name, "@contract/tool", "K4: name");
    assert_eq!(metadata.version, "1.1.0", "K4: version");
    assert_eq!(metadata.sha256, sha256_hex(wasm), "K4: sha256");
    assert_eq!(metadata.size_bytes, wasm.len() as u64, "K4: size_bytes");
    assert_eq!(
        metadata.manifest,
        serde_json::to_string(declaration).unwrap(),
        "K4: the manifest is the exact JSON uploaded"
    );
    assert_eq!(metadata.description, "Contract tool", "K4: description");
    assert_eq!(metadata.keywords, ["contract"], "K4: keywords");
    assert_eq!(metadata.key_id, key.key_id(), "K4: key_id");
    assert!(
        serde_json::from_str::<PackageSignature>(&metadata.signature).is_ok(),
        "K4: the signature parses as a PackageSignature"
    );
    assert!(!metadata.wasm_url.is_empty(), "K4: wasm_url");

    // K5
    let bytes = client
        .download(&metadata.wasm_url)
        .expect("K5: download of the metadata's wasm_url");
    assert_eq!(&bytes, wasm, "K5: the published bytes");
    assert_eq!(
        crate::verify_package_signature(&metadata, &bytes).expect("K5: the signature verifies"),
        TrustCheck::Verified {
            key_id: key.key_id()
        },
        "K5: verified by the publisher key"
    );

    // K6
    let error = client
        .metadata(&tool, &Version::new(9, 9, 9), registry)
        .unwrap_err();
    assert!(
        matches!(error, RegistryError::NotFound { .. }),
        "K6: metadata of an unpublished version is NotFound, not {error:?}"
    );

    // K7
    let manifest = serde_json::to_string(declaration).unwrap();
    let error = client
        .publish(
            wasm,
            declaration,
            &manifest,
            None,
            registry,
            Some(credential),
        )
        .unwrap_err();
    assert!(
        matches!(error, RegistryError::DuplicateVersion { .. }),
        "K7: publishing 1.1.0 again is DuplicateVersion, not {error:?}"
    );

    // K8
    let hits = client.search("contract", registry).expect("K8: search");
    assert_eq!(hits.len(), 1, "K8: one hit for \"contract\": {hits:?}");
    assert_eq!(hits[0].name, "@contract/tool", "K8: the hit's name");
    assert_eq!(hits[0].version, "1.1.0", "K8: the latest version");
    assert_eq!(hits[0].description, "Contract tool", "K8: the description");
    assert_eq!(
        client.search("a&b=c#d", registry).expect("K8: odd query"),
        vec![],
        "K8: a query cannot change the parameters"
    );

    // K9
    client
        .authenticate(registry, credential)
        .expect("K9: the credential authenticates");
    let wrong = RegistryCredential {
        alias: credential.alias.clone(),
        auth_method: AuthMethod::Bearer("not-a-token".to_string()),
    };
    let error = client.authenticate(registry, &wrong).unwrap_err();
    assert!(
        matches!(error, RegistryError::Unauthorized { .. }),
        "K9: a token the registry does not know is Unauthorized, not {error:?}"
    );

    // K10
    let next = contract_declaration("2.0.0");
    let manifest = serde_json::to_string(&next).unwrap();
    let error = client
        .publish(b"\0asm next", &next, &manifest, None, registry, None)
        .unwrap_err();
    assert!(
        matches!(error, RegistryError::Unauthorized { .. }),
        "K10: publishing with no credential is Unauthorized, not {error:?}"
    );
}
