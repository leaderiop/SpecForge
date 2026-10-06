use specforge_common::{Diagnostic, codes};
use specforge_protocol_types::ExtensionDeclaration;

/// One entry of the `providers` array in specforge.json (ADR 0004 D3-c):
/// the scheme it serves, its alias, the extension that implements it, and
/// that extension's settings. Two instances of one provider use two
/// schemes.
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderConfig {
    pub scheme: String,
    pub alias: String,
    pub extension: String,
    pub settings: serde_json::Map<String, serde_json::Value>,
}

/// Entry in the scheme registry mapping a URI scheme to a provider.
#[derive(Debug, Clone)]
pub struct SchemeRegistryEntry {
    pub scheme: String,
    /// The provider's alias.
    pub provider_name: String,
    pub extension_name: String,
}

/// Registry of URI schemes to provider mappings.
#[derive(Debug, Default)]
pub struct ProviderSchemeRegistry {
    pub entries: Vec<SchemeRegistryEntry>,
}

impl ProviderSchemeRegistry {
    pub fn find_by_scheme(&self, scheme: &str) -> Option<&SchemeRegistryEntry> {
        self.entries.iter().find(|e| e.scheme == scheme)
    }
}

fn provider_warning(message: String, suggestion: &str) -> Diagnostic {
    Diagnostic::new(codes::W118, message).with_suggestion(suggestion.to_string())
}

/// Parse the specforge.json `providers` array, in declaration order: each
/// entry needs `scheme`, `alias` and `extension`; `settings` is optional.
/// An entry missing one, or a `providers` value that is not an array of
/// objects, is W118.
pub fn load_provider_configurations(
    config: &serde_json::Value,
) -> (Vec<ProviderConfig>, Vec<Diagnostic>) {
    let mut providers = Vec::new();
    let mut diagnostics = Vec::new();

    let arr = match config.get("providers") {
        None | Some(serde_json::Value::Null) => return (providers, diagnostics),
        Some(serde_json::Value::Array(arr)) => arr,
        Some(_) => {
            diagnostics.push(provider_warning(
                "\"providers\" must be an array of {scheme, alias, extension, settings} entries"
                    .to_string(),
                "write providers as [{\"scheme\": \"gh\", \"alias\": \"main\", \"extension\": \"@acme/gh\"}]",
            ));
            return (providers, diagnostics);
        }
    };

    for (i, entry) in arr.iter().enumerate() {
        let text = |key: &str| {
            entry
                .get(key)
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        };
        let missing: Vec<&str> = ["scheme", "alias", "extension"]
            .into_iter()
            .filter(|key| text(key).is_none())
            .collect();
        if !missing.is_empty() {
            diagnostics.push(provider_warning(
                format!("providers[{i}]: missing {}", missing.join(", ")),
                "each provider needs a scheme, an alias and the extension that implements it",
            ));
            continue;
        }
        let settings = match entry.get("settings") {
            None => serde_json::Map::new(),
            Some(serde_json::Value::Object(settings)) => settings.clone(),
            Some(_) => {
                diagnostics.push(provider_warning(
                    format!("providers[{i}]: \"settings\" must be an object"),
                    "put the provider's own settings in a \"settings\" object",
                ));
                continue;
            }
        };
        providers.push(ProviderConfig {
            scheme: text("scheme").unwrap_or_default(),
            alias: text("alias").unwrap_or_default(),
            extension: text("extension").unwrap_or_default(),
            settings,
        });
    }

    (providers, diagnostics)
}

/// Why a configured provider's scheme is not registered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderStatus {
    Registered,
    /// Its extension is not loaded (not enabled, or failed to load).
    ExtensionNotLoaded,
    /// Its extension is loaded but contributes no providers.
    NotAProvider,
    /// An earlier provider already registered the scheme (E057).
    SchemeTaken,
}

impl ProviderStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderStatus::Registered => "registered",
            ProviderStatus::ExtensionNotLoaded => "extension_not_loaded",
            ProviderStatus::NotAProvider => "not_a_provider",
            ProviderStatus::SchemeTaken => "scheme_taken",
        }
    }
}

/// Register each configured provider's scheme to the extension it names,
/// in declaration order: the first provider to declare a scheme wins it,
/// and a later one is E057. A provider whose extension is not loaded, or
/// loaded but contributing no providers, is W118. Returns each provider's
/// status alongside, in declaration order.
pub fn register_provider_schemes(
    providers: &[ProviderConfig],
    declarations: &[ExtensionDeclaration],
) -> (ProviderSchemeRegistry, Vec<Diagnostic>) {
    let (registry, _, diagnostics) = register_provider_schemes_with_status(providers, declarations);
    (registry, diagnostics)
}

/// [`register_provider_schemes`], with each provider's status.
pub fn register_provider_schemes_with_status(
    providers: &[ProviderConfig],
    declarations: &[ExtensionDeclaration],
) -> (ProviderSchemeRegistry, Vec<ProviderStatus>, Vec<Diagnostic>) {
    let mut registry = ProviderSchemeRegistry::default();
    let mut statuses = Vec::with_capacity(providers.len());
    let mut diagnostics = Vec::new();

    for provider in providers {
        let declaration = declarations.iter().find(|d| d.name() == provider.extension);
        let status = match declaration {
            None => {
                diagnostics.push(provider_warning(
                    format!(
                        "provider '{}' names extension '{}', which is not loaded",
                        provider.alias, provider.extension
                    ),
                    "enable and install the extension: specforge add <extension>",
                ));
                ProviderStatus::ExtensionNotLoaded
            }
            Some(d) if !d.contribution_flags().providers => {
                diagnostics.push(provider_warning(
                    format!(
                        "provider '{}' names extension '{}', which contributes no providers",
                        provider.alias, provider.extension
                    ),
                    "name the extension that implements this provider",
                ));
                ProviderStatus::NotAProvider
            }
            Some(_) => match registry.find_by_scheme(&provider.scheme) {
                Some(first) => {
                    diagnostics.push(
                        Diagnostic::new(
                            codes::E057,
                            format!(
                                "scheme '{}' of provider '{}' ({}) is already registered by provider '{}' ({})",
                                provider.scheme,
                                provider.alias,
                                provider.extension,
                                first.provider_name,
                                first.extension_name
                            ),
                        )
                        .with_suggestion("give each provider instance its own scheme".to_string()),
                    );
                    ProviderStatus::SchemeTaken
                }
                None => {
                    registry.entries.push(SchemeRegistryEntry {
                        scheme: provider.scheme.clone(),
                        provider_name: provider.alias.clone(),
                        extension_name: provider.extension.clone(),
                    });
                    ProviderStatus::Registered
                }
            },
        };
        statuses.push(status);
    }

    (registry, statuses, diagnostics)
}
