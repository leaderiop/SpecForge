//! Wat-built `specforge:bridge` components for the runtime's tests: one bridge whose `call` runs a test's
//! body and answers a fixed string, and the guests that import what a test names (ADR 0326 D1a).

/// A `specforge:bridge` component whose `call` runs `body` (wat
/// instructions, with the locals `$left` and `$old` free to use), then
/// answers `Ok(answer)`. `outside` is component-level wat placed before the
/// core module (imports, other core modules and instances). `host` is wat
/// placed first in the core module: its imports, all from a module `"host"`
/// that `outside` defines as the core instance `$host`, then any other
/// declaration.
///
/// The result is lowered through a return area at offset 0: the `ok`
/// discriminant (0), the answer's pointer (16) and its length. The answer
/// itself sits at offset 16 and the bump allocator (`realloc`, for the
/// arguments the host lowers) starts at 8192.
pub fn custom_bridge(outside: &str, host: &str, body: &str, answer: &str) -> Vec<u8> {
    let escaped: String = answer.bytes().map(|byte| format!("\\{byte:02x}")).collect();
    let with = if outside.contains("(core instance $host") {
        r#"(with "host" (instance $host))"#
    } else {
        ""
    };
    let wat = r#"
(component
  (type $call_sig
    (func (param "name" string) (param "export-name" string) (param "input" (list u8))
      (result (result (list u8) (error string)))))
  OUTSIDE
  (core module $core
    IMPORTS
    (memory (export "memory") 1)
    (data (i32.const 16) "ANSWER")
    (global $heap (mut i32) (i32.const 8192))
    (func $realloc (export "realloc")
      (param $old_ptr i32) (param $old_size i32) (param $align i32) (param $new_size i32)
      (result i32)
      (local $ret i32)
      (local.set $ret
        (i32.and
          (i32.add (i32.add (global.get $heap) (local.get $align)) (i32.const 15))
          (i32.const -16)))
      (global.set $heap (i32.add (local.get $ret) (local.get $new_size)))
      (local.get $ret))
    (func (export "call") (param i32 i32 i32 i32 i32 i32) (result i32)
      (local $left i32)
      (local $old i32)
      BODY
      (i32.store (i32.const 0) (i32.const 0))
      (i32.store (i32.const 4) (i32.const 16))
      (i32.store (i32.const 8) (i32.const ANSWER_LEN))
      (i32.const 0)))
  (core instance $inst (instantiate $core WITH))
  (func $lifted (type $call_sig)
    (canon lift
      (core func $inst "call")
      (memory (core memory $inst "memory"))
      (realloc (core func $inst "realloc"))
      string-encoding=utf8))
  (export "call" (func $lifted)))
"#
    .replace("OUTSIDE", outside)
    .replace("IMPORTS", host)
    .replace("WITH", with)
    .replace("BODY", body)
    .replace("ANSWER_LEN", &answer.len().to_string())
    .replace("ANSWER", &escaped);
    wat::parse_str(wat).expect("the bridge component's wat parses")
}

/// The handshake every export of an importing guest answers: `@acme/importing` speaking `protocol`.
fn handshake(protocol: &str) -> String {
    format!(
        r#"{{"protocol_version":"{protocol}","name":"@acme/importing","version":"1.0.0","peer_dependencies":[],"sandbox_policy":null}}"#
    )
}

/// A bridge component importing each of `imports` as an instance exporting one function `f`, whose every export
/// answers the handshake of `@acme/importing` speaking `protocol`.
pub fn importing(imports: &[&str], protocol: &str) -> Vec<u8> {
    let outside: String = imports
        .iter()
        .map(|import| format!(r#"(import "{import}" (instance (export "f" (func))))"#))
        .collect();
    custom_bridge(&outside, "", "", &handshake(protocol))
}

/// A bridge component importing `import` as an empty instance: wasmtime links it with nothing to provide.
pub fn importing_empty(import: &str) -> Vec<u8> {
    let outside = format!(r#"(import "{import}" (instance))"#);
    custom_bridge(
        &outside,
        "",
        "",
        &handshake(specforge_protocol_types::PROTOCOL_VERSION),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasmtime::component::Component;
    use wasmtime::{Config, Engine};

    fn import_names(bytes: &[u8]) -> Vec<String> {
        let engine = Engine::new(&Config::new()).expect("engine initializes");
        let component = Component::from_binary(&engine, bytes).expect("the fixture compiles");
        component
            .component_type()
            .imports(&engine)
            .map(|(name, _)| name.to_owned())
            .collect()
    }

    #[test]
    fn importing_builds_a_component_that_names_each_import() {
        let names = ["specforge:host/http@9.9.0", "acme:x/y@1.0.0"];
        assert_eq!(import_names(&importing(&names, "1.0.0")), names);
        assert_eq!(
            import_names(&importing_empty("specforge:host/http@9.9.0")),
            ["specforge:host/http@9.9.0"]
        );
    }
}
