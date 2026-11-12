//! The registry's diagnostic codes (`R…`, the extension registry and its client): a table `catalog_merged.rs` merges
//! into `CATALOG`, sorted by code. Core codes: each has its constant in [`codes`](crate::codes).

catalog! {
    R_AUTH_020 = "R-AUTH-020": Error core,
        "Stored registry token expired",
        "The token `specforge login` stored for this registry expired at the reported time. Log in again with a new token: `specforge login --registry <alias> --token <NEW_TOKEN>`.";
    R_AUTH_021 = "R-AUTH-021": Error core,
        "Stored registry token unreadable",
        "The secret `specforge login --token` kept for this registry (a file under `~/.specforge/secrets/`) is missing or can't be read, although the credentials file refers to it. Log in again: `specforge login --registry <alias> --token <NEW_TOKEN>`. The file is 0600 and its directory 0700 on unix only; on Windows nothing sets a mode, so the secret relies on the user profile directory's default ACL (private to the user).";
    R_LOGIN_001 = "R-LOGIN-001": Error core,
        "No single login token source",
        "`specforge login` needs exactly one of `--token <TOKEN>`, `--token-env <VAR>` or `--token-file <PATH>`; none or several were given.";
    R_LOGIN_002 = "R-LOGIN-002": Error core,
        "Login token not stored",
        "`specforge login` couldn't write the token's secret file under `~/.specforge/secrets/`. Check that `~/.specforge` is writable.";
    R_OPS_001 = "R-OPS-001": Error core,
        "No registry for the package",
        "No configured registry serves this package: none has a scope that matches it, and none is marked as the default (or no registries are configured at all). Add a `registries` entry to `specforge.json` with a matching scope, or mark one `\"default_registry\": true`. `specforge add`, `update` and `publish` (and MCP `add_extension`) choose the registry by this one rule and ask no other registry; a refused name sends no request.";
    R_OPS_002 = "R-OPS-002": Error core,
        "Package integrity check failed",
        "The SHA-256 hash of the downloaded package doesn't match the hash the registry published for it, so the download is corrupt or was tampered with. Retry the download; if it keeps failing, don't install the package.";
    R_OPS_003 = "R-OPS-003": Error core,
        "Manifest not serializable",
        "`specforge publish` couldn't serialize the extension manifest to JSON for the upload. This is a SpecForge bug, not a mistake in your manifest; please report it.";
    R_OPS_004 = "R-OPS-004": Error core,
        "Package manifest unreadable",
        "The registry served no manifest for the package, or one that isn't a valid extension manifest. The manifest declares the package's peer dependencies, which `specforge add` checks against the installed extensions before installing anything, so a package whose manifest can't be read is refused rather than treated as having no peers. Nothing is installed and no publisher key is pinned. Don't install the package, and check the registry.";
    R_RES_001 = "R-RES-001": Error core,
        "Package not in the registry",
        "The registry has no package with this name. Check the package name and which registry the configuration sends it to.";
    R_RES_002 = "R-RES-002": Error core,
        "No published versions",
        "The registry lists the package, but no version of it (or no version with a valid semver number). Ask the publisher to publish a release, or install from another source.";
    R_RES_003 = "R-RES-003": Error core,
        "Invalid version range",
        "The version after `@` isn't `latest`, a full version or valid SemVer requirement syntax, so no registry was asked. Use a version such as `1.2.0`, a requirement such as `^1.0`, `~2.3`, `1.x` or `>=1.0.0, <2.0.0`, or `latest`.";
    R_RES_004 = "R-RES-004": Error core,
        "No version satisfies the range",
        "The registry has versions of the package, but none inside the requested range; the message lists the available ones (for a builtin, the one version this specforge embeds). Widen the range, or pick one of the listed versions; for a builtin, drop the version or use the specforge release that embeds the one you need.";
    R_RES_005 = "R-RES-005": Error core,
        "Unresolvable version diamond",
        "Several extensions require this package in ranges that no single published version satisfies (see ADR 0001). Upgrade the requirer with the narrowest range, or pin a compatible version manually.";
    R_RES_006 = "R-RES-006": Error core,
        "Locked peer breaks a version diamond",
        "An extension being installed or updated requires a peer at a version the lock does not have, or a locked extension requires a version of the package being installed or updated that it would not have. The message names a version of the peer that would satisfy every requirer. Install the peer at that version (`specforge add @scope/peer@<version>`) and retry; when a locked extension holds the peer back, `specforge update --major` moves them together.";
    R_TRUST_001 = "R-TRUST-001": Error core,
        "Unsigned package",
        "The registry package carries no publisher signature, so where it came from can't be verified. Install it anyway only if you trust the source, with `--allow-unsigned`; the MCP server never accepts an unsigned package.";
    R_TRUST_002 = "R-TRUST-002": Error core,
        "Invalid package signature",
        "The package's publisher signature doesn't verify: the Wasm binary or the manifest isn't what the publisher signed. Don't install the package. `--allow-unsigned` doesn't accept it. (A signature that isn't one at all makes the registry's answer unreadable: R009.)";
    R_TRUST_003 = "R-TRUST-003": Error core,
        "Publisher key changed",
        "The package is signed with a different key than the one expected for it: the key pinned for it when you first installed it (trust on first use), or, when you have no pin for it, the key the project's `specforge.lock` records. That can be a key rotation or a compromised publisher. If you trust the new key, accept it with `specforge add` or `specforge update` (yes at the prompt, or `--yes`), or add it to `trusted_keys`; the MCP server never accepts a key change.";
    R_TRUST_004 = "R-TRUST-004": Error core,
        "Registry answer mismatch",
        "The registry's answer doesn't match: it, or the manifest it serves, describes another package or version than the one requested, or the binary declares other than the served declaration. Don't install the package, and check the registry.";
    R_TRUST_005 = "R-TRUST-005": Error core,
        "Publisher key denied",
        "The package is signed with a key listed in `denied_keys` in your known-keys file. Remove the key from `denied_keys` only if you trust it again.";
    R_TRUST_006 = "R-TRUST-006": Error core,
        "Known-keys file unusable",
        "Your known-keys file (`~/.specforge/known-keys.json`) can't be read as the store of publisher pins, or the pins an install accepted can't be saved to it. Nothing is installed either way: a store read as empty would trust any key and lose your `denied_keys`, and an install whose pin can't be saved is put back. Fix the file's JSON, or move it aside (every package is then trusted on first use again), and check the permissions on the file and its directory.";
    R_TRUST_007 = "R-TRUST-007": Error core,
        "Unsigned package for a pinned publisher",
        "The registry package carries no publisher signature, but a publisher key is expected for its name: the key you pinned when you first installed it, or, when you have no pin for it, the key the project's `specforge.lock` records. A package its publisher signs does not arrive unsigned, so this can be a mirror or a compromised registry serving other bytes. Nothing is downloaded, and neither `--allow-unsigned`, `--yes`, an interactive yes nor `trusted_keys` accepts it. Check the registry that serves the name. If its publisher really stopped signing it, remove the extension (`specforge remove <name>`) and its pin in `~/.specforge/known-keys.json`, then add it again.";
    R001: Error core,
        "Registry authentication failed",
        "The registry rejected the request as unauthenticated (HTTP 401). Log in again with `specforge login --registry <alias> --token <TOKEN>`. `specforge publish` refuses before any request when there is no credential for the registry that serves the package: none stored for its alias and `SPECFORGE_REGISTRY_TOKEN` unset or blank.";
    R002: Error core,
        "Registry access forbidden",
        "The registry accepted the credentials but refused the request (HTTP 403). Check your permissions for the registry or the package scope.";
    R003: Warning core,
        "Registry rate limit",
        "The registry is rate limiting requests (HTTP 429); the message says how long to wait. Retry after that delay.";
    R004: Error core,
        "Registry request timed out",
        "A request to the registry didn't answer in time. Check the network connection and the registry URL, or try again later.";
    R005: Error core,
        "Registry network error",
        "A request to the registry failed at the network level. Check the network connection and the registry URL.";
    R006: Error core,
        "Package version not found",
        "The registry answered that it has no such package or version (HTTP 404 with the registry protocol's NOT_FOUND error). Check the package name and version.";
    R007: Error core,
        "Version already published",
        "`specforge publish` tried to publish a version that already exists for the package, and published versions are immutable. Bump the version in the manifest and publish again.";
    R008: Error core,
        "Registry refused the request",
        "The registry read the request and refused it (HTTP 400), naming its reason and code: for a publish, a package that isn't signed or whose signature isn't one or doesn't verify over the uploaded binary and manifest (INVALID_SIGNATURE), a binary that isn't a Wasm module, or a manifest that isn't an extension declaration or names another package. Nothing was stored, and the version can still be published. Fix what the message names and try again; sending the same request again is refused the same way.";
    R009: Error core,
        "Registry reply unreadable",
        "The registry's answer isn't one the registry protocol defines for that request: a body that isn't the request's JSON (such as a publisher signature that isn't an Ed25519 signature object, or a publish receipt that describes another upload than the one sent), a name that is no package name, a version that is no SemVer version, a digest that is not a SHA-256 in lowercase hex or a token expiry that is not an RFC 3339 timestamp (each refused before anything is downloaded or kept), a 404 without the protocol's NOT_FOUND error (no registry serves that address: the configured `url` isn't a SpecForge registry's root), a download URL that is neither an `http`/`https` URL nor a path at the registry's root, or an answer longer than the request admits (a download longer than the `size_bytes` its reply declares, or any answer over 64 MiB, the largest package a registry accepts; it is refused before more of it is read). The registry is broken, or the URL isn't a SpecForge registry's root. Nothing is installed and no key is pinned, and `--allow-unsigned` doesn't accept such a package. Check the registry and its `url` in `specforge.json`.";
    R010: Error core,
        "Registry token variable not set",
        "The registry's credential is a reference to an environment variable (`specforge login --token-env`, or a `token_env` entry in `~/.specforge/credentials.json`) that isn't set, or is blank, where the command runs. Set it, or log in again with another source.";
    R011: Error core,
        "Registry token file unreadable",
        "The registry's credential is a reference to a token file (`specforge login --token-file`, or a `token_file` entry) that can't be read, or is empty. Check the path and its permissions, or log in again with another source.";
    R012: Error core,
        "Credentials file unreadable",
        "`~/.specforge/credentials.json` can't be read, or isn't in the expected format. Check its permissions, or delete it and log in again.";
    R013: Error core,
        "Credentials file not writable",
        "The credentials couldn't be saved: creating `~/.specforge`, serializing the credentials, or writing `~/.specforge/credentials.json` failed. Check the permissions on `~/.specforge`.";
    R014: Error core,
        "Locked package served as other bytes",
        "The registry serves the version of a package that `specforge.lock` pins, but not the binary the lock pins: its SHA-256 is another. A published version never changes (R007), so the registry, or the address `specforge.json` sends the package to, now serves something else under that version. Nothing was downloaded or installed, no publisher key was pinned or asked about, and the lock is left as it was; `specforge add --dry-run` reports it too. Check the registry. If you trust the bytes it now serves, remove the extension (`specforge remove <name>`) and add it again, which pins them.";
}
