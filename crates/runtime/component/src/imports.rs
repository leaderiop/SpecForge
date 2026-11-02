//! A component is admitted by its import names before it is instantiated (ADR 0326 D1a): it imports only `wasi:*`
//! and the `specforge:host` interfaces this host links.

use specforge_wasm::LoadError;

/// The `specforge:host` interfaces this host links, `iface@version`. None at protocol 1.x; the ports round adds
/// each port as it links it.
pub(crate) const HOST_PORTS: &[&str] = &[];

/// Refuses a component by its import names: a foreign import first (no specforge loads it), then a
/// `specforge:host` interface not in [`HOST_PORTS`] (a guest built for a newer host). Every offending name is listed,
/// in import order. A `wasi:*` import passes: one the linker lacks still fails at instantiation, naming it.
pub(crate) fn admit<'a>(names: impl Iterator<Item = &'a str>) -> Result<(), LoadError> {
    let (mut foreign, mut unknown) = (Vec::new(), Vec::new());
    for name in names {
        let package = name.split_once('/').map_or(name, |(package, _)| package);
        if package.starts_with("wasi:") {
            continue;
        }
        if package != "specforge:host" {
            foreign.push(name.to_string());
        } else if !HOST_PORTS.contains(&name) {
            unknown.push(name.to_string());
        }
    }
    if !foreign.is_empty() {
        return Err(LoadError::ForeignImports(foreign));
    }
    if !unknown.is_empty() {
        return Err(LoadError::UnknownPorts(unknown));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasi_imports_pass() {
        let names = ["wasi:io/poll@0.2.6", "wasi:cli/environment@0.2.0"];
        assert_eq!(admit(names.into_iter()), Ok(()));
    }

    #[test]
    fn an_unknown_host_port_is_named_with_its_version() {
        let names = ["wasi:io/poll@0.2.6", "specforge:host/http@9.9.0"];
        assert_eq!(
            admit(names.into_iter()),
            Err(LoadError::UnknownPorts(vec![
                "specforge:host/http@9.9.0".to_string()
            ]))
        );
    }

    #[test]
    fn every_unknown_port_is_listed_in_import_order() {
        let names = ["specforge:host/http@9.9.0", "specforge:host/fs@9.9.0"];
        assert_eq!(
            admit(names.into_iter()),
            Err(LoadError::UnknownPorts(vec![
                "specforge:host/http@9.9.0".to_string(),
                "specforge:host/fs@9.9.0".to_string(),
            ]))
        );
    }

    #[test]
    fn a_foreign_import_wins_over_an_unknown_port() {
        let names = ["specforge:host/http@9.9.0", "acme:x/y@1.0.0"];
        assert_eq!(
            admit(names.into_iter()),
            Err(LoadError::ForeignImports(vec![
                "acme:x/y@1.0.0".to_string()
            ]))
        );
    }

    #[test]
    fn an_unversioned_host_import_is_unknown() {
        assert_eq!(
            admit(["specforge:host/http"].into_iter()),
            Err(LoadError::UnknownPorts(vec![
                "specforge:host/http".to_string()
            ]))
        );
    }
}
