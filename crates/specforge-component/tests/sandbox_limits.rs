//! The limits the component runtime holds an extension to (ADR 0037).
//!
//! The guests are hand-written `specforge:bridge` components (like
//! `epoch_deadline.rs`'s spin component), built by [`bridge_component`]:
//! each call spins a counted loop, grows linear memory, and answers a
//! fixed string.

use specforge_component::ComponentRuntime;
use specforge_wasm::ExtensionCalls;
use specforge_wasm::runtime::{WasmCallResult, WasmRuntime};

/// A handshake declaring a memory limit of 1 MB, the answer of every export
/// of the growing guest.
const DECLARES_ONE_MB: &str = r#"{"protocol_version":"1.0.0","name":"@repro/grow","version":"0.1.0","contribution_flags":{},"peer_dependencies":[],"sandbox_policy":{"max_memory_mb":1}}"#;

/// A `specforge:bridge` component whose every export (its `call` func, the
/// canonical-ABI signature `(name, export-name, input) -> result<list<u8>,
/// string>`):
///
/// - runs a counting loop of `spin` iterations;
/// - grows its linear memory by `pages` 64 KiB pages, writing every byte of
///   them when `fill` is set (so the growth costs real memory; without it
///   the pages stay virtual and a test is cheap);
/// - answers `Ok(answer)`.
///
/// The result is lowered through a return area at offset 0: the `ok`
/// discriminant (0), the answer's pointer (16) and its length. The answer
/// itself sits at offset 16 and the bump allocator (`realloc`, for the
/// arguments the host lowers) starts at 8192.
fn bridge_component(pages: u32, spin: u32, fill: bool, answer: &str) -> Vec<u8> {
    let escaped: String = answer.bytes().map(|byte| format!("\\{byte:02x}")).collect();
    let fill_pages = if fill {
        "(memory.fill (i32.mul (local.get $old) (i32.const 65536)) (i32.const 1)
          (i32.mul (i32.const PAGES) (i32.const 65536)))"
    } else {
        ""
    };
    let wat = r#"
(component
  (type $call_sig
    (func (param "name" string) (param "export-name" string) (param "input" (list u8))
      (result (result (list u8) (error string)))))
  (core module $core
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
      (local.set $left (i32.const SPIN))
      (block $done
        (loop $again
          (br_if $done (i32.eqz (local.get $left)))
          (local.set $left (i32.sub (local.get $left) (i32.const 1)))
          (br $again)))
      (if (i32.ne (i32.const PAGES) (i32.const 0))
        (then
          (local.set $old (memory.grow (i32.const PAGES)))
          FILL))
      (i32.store (i32.const 0) (i32.const 0))
      (i32.store (i32.const 4) (i32.const 16))
      (i32.store (i32.const 8) (i32.const ANSWER_LEN))
      (i32.const 0)))
  (core instance $inst (instantiate $core))
  (func $lifted (type $call_sig)
    (canon lift
      (core func $inst "call")
      (memory (core memory $inst "memory"))
      (realloc (core func $inst "realloc"))
      string-encoding=utf8))
  (export "call" (func $lifted)))
"#
    .replace("FILL", fill_pages)
    .replace("ANSWER_LEN", &answer.len().to_string())
    .replace("ANSWER", &escaped)
    .replace("PAGES", &pages.to_string())
    .replace("SPIN", &spin.to_string());
    wat::parse_str(wat).expect("the bridge component's wat parses")
}

fn answered(result: &WasmCallResult) -> bool {
    matches!(result, WasmCallResult::Ok(_))
}

fn trap_kind(result: &WasmCallResult) -> &str {
    match result {
        WasmCallResult::Trap(trap) => &trap.kind,
        WasmCallResult::Ok(_) => panic!("expected a trap, got Ok"),
    }
}

/// Pin of today: nothing reads `max_memory_mb`. An extension declaring 1 MB
/// grows by 128 MiB per call, and every call answers.
#[test]
fn pin_a_declared_memory_limit_is_not_enforced() {
    let runtime = ComponentRuntime::new();
    runtime
        .load_module_bytes(
            "@repro/grow",
            &bridge_component(2048, 0, false, DECLARES_ONE_MB),
        )
        .expect("the growing guest instantiates");
    ExtensionCalls::new(&runtime)
        .handshake("@repro/grow")
        .expect("the handshake answers");
    for _ in 0..2 {
        let result = runtime.call_export("@repro/grow", "anything", b"{}");
        assert!(answered(&result), "growth past 1 MB answers: {result:?}");
    }
}

/// Pin of today: the fuel budget is set when the instance is made, so the
/// calls of one instance share it. A budget of 1,000,000 serves two calls
/// of 50,000 iterations and the third traps, anonymously.
#[test]
fn pin_fuel_is_spent_across_calls() {
    let runtime = ComponentRuntime::new().with_fuel_limit(1_000_000);
    runtime
        .load_module_bytes("@repro/spin", &bridge_component(0, 50_000, false, "{}"))
        .expect("the spinning guest instantiates");
    for call in 1..=2 {
        let result = runtime.call_export("@repro/spin", "anything", b"{}");
        assert!(answered(&result), "call {call} answers: {result:?}");
    }
    let third = runtime.call_export("@repro/spin", "anything", b"{}");
    assert_eq!(trap_kind(&third), "call_failed");
}
