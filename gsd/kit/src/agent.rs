//! Agents shared by the extensions.

use specforge_extension_sdk::prelude::*;

/// The notice every ported agent carries: its upstream file (`path`, as upstream's tree names it), version and licence.
pub fn attribution(path: &str) -> Attribution {
    Attribution {
        upstream: "gsd-core".to_string(),
        path: path.to_string(),
        version: "v1.18.0".to_string(),
        license: "MIT".to_string(),
        copyright: "Copyright (c) 2026 Open GSD".to_string(),
    }
}
