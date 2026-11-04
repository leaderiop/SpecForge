// The host ports a guest reaches an effect through (ADR 0335 D4)
//
// An extension has two faces for effects: what the host serves a call as declared inputs, and the four granted
// ports specforge:host/{http,fs,process,git}@0.1.0. Every instance links the four; what one call may do is decided
// per call, by its kind and its extension's grant. Nothing ever prompts mid-call. A port call is a synchronous leaf
// operation of the call that made it: it produces no event of its own, and an effect's record is the effects
// journal's (ADR 0337).

use "invariants/wasm"
use "invariants/zero-entity-core"
use "ports/outbound"
use "types/wasm"

defaults behavior {
  features   [wasm_host_function_api]
  invariants [wasm_sandbox_integrity, host_function_type_safety, extension_isolation]
  types      [HostPortDenial]
  ports      [WasmRuntime]
}

behavior link_host_ports "Link Host Ports" {
  category command
  requires {
    instance_built "the host is building an instance of a loaded component"
  }
  ensures {
    four_linked           "every instance links specforge:host/{http,fs,process,git}@0.1.0, whatever its grant"
    pure_kind_denied      "a call kind the gate keeps from the port is answered denied(call-kind) and nothing runs"
    never_prompted        "no port call prompts: an approval is settled before the call's first effect"
    imports_what_it_calls "a guest imports only the interfaces it calls, so a guest that calls none imports none"
  }
  contract """
    Every instance MUST link the four specforge:host ports and their
    shared types interface. A port call from a call kind the gate
    (ADR 0335 D2.2) keeps from that port MUST be answered denied(call-kind)
    without running. A port call MUST NOT prompt: what a call may do is
    settled before it starts.
  """
  verify unit "a pure call that calls a port is answered denied(call-kind) and no port runs"
  verify integration "every instance links the four ports, and a guest that calls none imports none"
}

behavior grant_a_host_port_call "Grant a Host Port Call" {
  category command
  ports    [WasmRuntime, Http, Fs, Process, Git]
  requires {
    call_lent "an effectful call was lent its extension's grant and the ports"
  }
  ensures {
    inside_runs      "a request inside the grant runs through the call's effects mode and journal"
    outside_denied   "a request outside the grant is answered denied(scope) and reported as E090"
    answer_held      "every byte payload of an answer is held to a quarter of the guest's memory and says when it was cut"
    no_secret_enters "no secret value enters a guest: a request cannot carry one, an answer shows none, and a program a guest runs does not inherit one"
    no_host_opt_in   "a guest cannot open what only the host may: private addresses, the program's PATH or loader, a commit's withheld file"
  }
  contract """
    An effectful call (a command, an MCP tool or resource, a collector) runs
    a port only inside its grant (ADR 0335 D2). A request the gate, the
    declaration or the ceiling refuses MUST be answered denied(scope) and
    reported as E090 against the extension; one that lacks this machine's
    approval MUST be answered denied(scope) without E090 (ADR 0335 D2.7).
    Every byte payload of an answer MUST be held to a quarter of the guest's
    max_memory_mb and flagged truncated when cut, so an answer never traps
    the guest's memory. A secret MUST stay on the host (ADR 0288, ADR 0335
    D6): the host attaches it to a granted request, the guest's request has
    no place for one, and a program the guest runs MUST NOT inherit the
    variable a declared secret names.
  """
  verify unit "a command's port call inside its grant reaches the port"
  verify unit "a port call outside the grant is answered denied(scope) and reported as E090"
  verify unit "a port's answer is held to a quarter of the guest's memory and says it was truncated"
  verify unit "the in-process runtime answers a port call as the component runtime does"
  verify unit "a secret the host attaches never shows in what a guest is answered"
  verify unit "a program a guest runs inherits no declared secret and cannot set its PATH or loader"
  verify unit "a guest cannot open the private-address opt-in or read a withheld file at a commit"
  verify integration "the sandbox probe reaches each of the four ports through its grant: a command three, a collector process"
}

behavior bound_a_host_port_call "Bound a Host Port Call" {
  category command
  requires {
    call_running "a guest is running under its limits and has called a port"
  }
  ensures {
    guest_time_only "the epoch deadline counts the guest's own time: a port call's time is not spent from it"
    port_bounded    "a port call is bounded by its own deadline and the call's port budget"
    lock_free       "an effectful call of a guest that imports a port holds no lock of its extension while it runs"
  }
  contract """
    Epoch interruption MUST bound a call's guest time only (ADR 0037 D3): a
    port's time is bounded by the port's own deadline and a per-call port
    budget of 600 s. An effectful call of a guest that imports a port MUST
    run on its own instance, so it holds no lock of its extension. Fuel
    counts guest instructions only, so a work budget is untouched by a
    port's time.
  """
  verify unit "a port call that outlasts the guest's deadline does not trap the guest"
  verify unit "a guest that spins after a port call still traps at its own deadline"
  verify unit "a port call does not hold its extension's lock"
}

behavior refuse_an_unbacked_port_import "Refuse an Unbacked Port Import" {
  category command
  requires {
    declaration_read "the extension's declaration was read from its describe"
  }
  ensures {
    e089_refused "a port the component imports with no capability of that port declared fails the load with E089, every such port listed"
  }
  contract """
    An extension that imports a specforge:host port (types aside) MUST
    declare a capability of that port; otherwise it is not loaded (E089).
    A capability without an import is legitimate: declared probes use
    capabilities too.
  """
  verify unit "an extension that imports a port no capability backs is not loaded (E089)"
  verify integration "the environment load reports E089 naming the extension and the port"
}

behavior call_host_ports_from_the_sdk "Call Host Ports from the SDK" {
  category command
  requires {
    sdk_linked "an extension is built against the SDK"
  }
  ensures {
    typed_client   "host::{http,fs,process,git} call the ports in the protocol's types"
    outside_denied "a port called where no host serves it is answered denied(call-kind)"
    declared       "an extension declares each capability its imports use, with its port, scope, call kinds and reason"
  }
  contract """
    The SDK MUST offer one client per port in the protocol's types, over
    the one WIT package it ships. In a component each call is the import; in
    process (an extension's own tests) it is the host served on the thread,
    and with none it is answered denied(call-kind), the answer a pure call
    gets.
  """
  verify unit "every port request and answer survives the SDK's WIT conversion"
  verify unit "a port called outside any host is answered denied(call-kind)"
  verify unit "an extension declares a capability with its port, scope, call kinds and reason"
}
