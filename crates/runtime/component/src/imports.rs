//! A component is admitted by its import names before it is instantiated (ADR 0326 D1a): it imports only `wasi:*`
//! and the `specforge:host` interfaces this host links.

use specforge_wasm::LoadError;

/// The `specforge:host` interfaces this host links, `iface@version`: every instance has them (`ports::link`), and a
/// guest imports only those it calls.
pub(crate) const HOST_PORTS: &[&str] = &[
    "specforge:host/types@0.1.0",
    "specforge:host/http@0.1.0",
    "specforge:host/fs@0.1.0",
    "specforge:host/process@0.1.0",
    "specforge:host/git@0.1.0",
];

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
    fn a_known_host_port_passes() {
        let names = [
            "wasi:io/poll@0.2.6",
            "specforge:host/types@0.1.0",
            "specforge:host/http@0.1.0",
        ];
        assert_eq!(admit(names.into_iter()), Ok(()));
        assert_eq!(
            admit(["specforge:host/http@0.2.0"].into_iter()),
            Err(LoadError::UnknownPorts(vec![
                "specforge:host/http@0.2.0".to_string()
            ]))
        );
    }

    #[test]
    fn the_host_ports_are_the_wit_packages_interfaces() {
        let wit = std::fs::read_to_string(
            specforge_repo_root::repo_root().join("crates/runtime/extension-sdk/wit/host.wit"),
        )
        .expect("host.wit is the SDK's");
        let version = wit
            .lines()
            .find_map(|line| line.strip_prefix("package specforge:host@"))
            .and_then(|rest| rest.strip_suffix(';'))
            .expect("host.wit names its package");
        let mut interfaces: Vec<String> = wit
            .lines()
            .filter_map(|line| line.strip_prefix("interface "))
            .filter_map(|rest| rest.strip_suffix(" {"))
            .map(|interface| format!("specforge:host/{interface}@{version}"))
            .collect();
        let mut ports: Vec<String> = HOST_PORTS.iter().map(ToString::to_string).collect();
        interfaces.sort();
        ports.sort();
        assert_eq!(ports, interfaces);
    }

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
