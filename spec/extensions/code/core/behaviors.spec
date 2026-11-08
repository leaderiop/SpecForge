// @code/core behaviors (ADR 0329, ADR 0347)

use "types/wasm"
use "types/zero-entity-core"

behavior cc_declare_test_facts "Declare the Test Fact Kinds" {
  features [cc_test_facts]
  category query
  types    [EntityKindDescriptor, Fact, TestStatus]
  contract """
    @code/core MUST declare two collector-written kinds, neither scoped:
    test, under the schemes js and rust, with package, file, suite, name,
    status (an enum of the TestStatus names, the kind's result) and names; and
    test_run, under the scheme run, with test (a reference to the test it
    ran, edge run_of), status and duration_ms. It MUST catalogue the codes it
    reports, which print under the namespace code (ADR 0328 D2).
  """
  ensures {
    kinds_collected  "test is collected under js and rust, test_run under run"
    result_status    "test names status as its result"
    codes_namespaced "E101, W102, W401 and I402 print as code/E101, code/W102, code/W401 and code/I402"
  }
  verify unit "the kinds are collected with their schemes"
}

behavior cc_declare_code_facts "Declare the Item and Package Fact Kinds" {
  features [cc_code_facts]
  category query
  types    [EntityKindDescriptor, Fact]
  contract """
    @code/core 1.1 MUST declare test under the schemes js and rust, with a
    binary field and names a reference list of any kind (edge names); item
    under rust and js, with package (a reference to a package, edge
    in_package), file, kind (a list of the library's item kinds),
    visibility (public, crate, restricted or private), signature, doc,
    attrs, names (edge names) and test_id (a reference to a test, edge
    runs_as); and package under pkg, with name, version, edition,
    rust_version, engines, features, targets, path and depends_on (a
    reference list to packages, edge uses_package). Code/I402 MUST report a
    dangling name of a test or an item (ADR 0347 D11).
  """
  ensures {
    four_kinds     "test, test_run, item and package are collected with their schemes"
    item_edges     "item and package reference their facts by in_package, runs_as and uses_package"
    names_dangling "a test or item naming no entity is code/I402"
  }
  verify unit "the kinds are collected with their schemes"
  verify unit "item and package reference their facts by in_package, runs_as and uses_package"
  verify unit "test names is a reference list of any kind"
  verify unit "the item kind vocabulary is the librarys"
  verify unit "a test or item naming no entity is code/I402"
  verify unit "an item naming several entities is i402 listing them"
}

behavior cc_kind_result "A Collected Kind Names the Field That Holds Its Result" {
  features [cc_test_facts]
  category validation
  types    [EntityKindDescriptor, TestStatus]
  contract """
    A kind MAY name the field that holds its result (protocol 1.15.0). The
    registry MUST keep it only on a collector-written kind, naming an enum
    field the kind declares whose values are all TestStatus names; any other
    is W021 and the attribute is dropped, the kind kept.
  """
  ensures {
    kept_on_a_status_enum "a collected kind's status enum is kept as its result"
    refused_otherwise     "a result on an authored kind, on an undeclared or non-enum field, or with a value outside TestStatus is W021"
  }
  verify unit "a result field must be a status enum of a collector-written kind"
}

behavior cc_owner_pending "A Pass Entity Is Pending While Its Owner Is" {
  features [cc_owner_state]
  category query
  types    [EntityKindDescriptor, PassEntity]
  contract """
    A kind MAY declare complete_when, a field and the values that make its
    entities complete (protocol 1.15.0). The registry MUST keep it only
    when the field is an enum field the kind declares and every value is
    one of its values; any other is W021 and the declaration is dropped.
    The host MUST send a pass entity pending when its own kind's
    complete_when does not hold, or, when its kind declares none, when the
    nearest scope ancestor whose kind declares one does not; with no
    declaration in the chain it is not pending. It is decided on the
    snapshot the passes read, computed values included.
  """
  ensures {
    own_kind_decides  "an entity whose kind declares complete_when is pending while its field holds none of the values"
    scope_decides     "an entity whose kind declares none follows its nearest declaring scope ancestor"
    none_not_pending  "an entity with no declaration in its scope chain is not pending"
    malformed_refused "a complete_when naming no enum field of its kind, or a value outside it, is W021"
  }
  verify unit "a scoped entity is pending while its owner is"
  verify unit "an entity with no stated owner is not pending"
  verify unit "complete_when names an enum value of its kind"
}

behavior cc_check_fact_refs "Check Cited Fact References" {
  features [cc_test_facts, cc_owner_state]
  category validation
  types    [PassInput, PassEntity]
  contract """
    The check pass refs MUST judge each unheld reference an entity writes
    whose collected kinds (the kinds of its scheme its field admits) meet
    @code/core's kinds, once, at the span it is written at: nothing while
    the entity is pending; code/E101 when a served fact store holds a fact
    of one of those kinds in the reference's scheme, a test reference
    suggesting the closest held test of the same container; code/W102 when
    none does, naming the scheme, the kind and the pack that writes it
    (ADR 0347 D9). A held reference links and is never a finding, and
    neither is a fact nothing cites. A reference one of @code/core's facts
    holds that no fact or entity answers (unheld, or dangling: no entity,
    several, or one of another kind) is code/E101 addressed to its
    collector, at the span it was read at, whether or not a store serves
    its scheme.
  """
  ensures {
    e101_when_served   "a reference of a scheme a served store holds, naming no fact, is code/E101"
    w102_when_unserved "a reference of a scheme no served store holds is code/W102"
    pending_unchecked  "a pending entity's references are not checked"
    held_silent        "a reference a fact holds links and reports nothing"
  }
  verify integration "a fact ref to a missing test is code/E101"
  verify integration "a fact ref with no facts collected is code/W102"
  verify integration "a fact ref of a pending owner is unchecked"
  verify integration "a held fact ref links and reports nothing"
  verify unit "a fact's reference no fact or entity answers is code/E101 addressed to its collector, at its span"
  verify integration "a test run's dangling test reference is code/E101 at the fact, addressed to its collector"
  verify unit "a reference whose kinds no served store holds in its scheme is code/W102 naming the kind and its pack"
  verify unit "a reference naming no fact of a kind a served store holds in its scheme is code/E101"
  verify unit "a rust test ref beside rust items only is w102"
  verify unit "a facts own unheld reference is not judged"
  verify unit "a reference admitting none of its kinds is not judged"
  verify integration "a rust test ref beside only item facts is code/W102"
  verify integration "a missing item of a collected crate is code/E101"
  verify integration "an item ref links to its fact"
}

behavior cc_check_proof "Report a Failing Proof and a Test Naming No Entity" {
  features [cc_test_facts]
  category validation
  types    [PassInput, PassEntity]
  contract """
    The check pass proof MUST report code/W401 on each entity outside
    @code/core's kinds that is not pending and links a test whose status is
    failed, and code/I402 on each test or item whose names holds a dangling
    name: no entity of the project, or the short name of several.
  """
  ensures {
    w401_failing_proof "a complete owner linking a failed test is code/W401"
    pending_silent     "a pending owner linking a failed test is not reported"
    i402_dangling_name "a test naming an entity that does not exist is code/I402"
  }
  verify integration "a failing proof is code/W401"
  verify integration "a test naming a missing entity is code/I402"
}

behavior cc_mint_code_ids "Mint Every Code Id in One Library" {
  features [cc_code_facts]
  category computation
  types    [Fact]
  contract """
    specforge-code-facts MUST mint every test, item and package id and fact
    (ADR 0347): a JS test js:<package>/<file>#<suites > name>, a Rust test
    rust:<binary id>#<path>, a Rust item rust:<crate segment>::<path> with
    a member's owner written <Type> or <Type as Trait> (last path segment,
    generic arguments dropped), a JS item js:<package>/<file>::<name> with
    members joined by a dot, a package pkg:cargo/<name>, pkg:cargo/<name>#
    <dir> or pkg:npm/<name> with an npm scope's at sign percent-encoded. An
    item id MUST never hold a hash sign; every id MUST parse back to itself;
    a SCIP symbol of rust-analyzer or scip-typescript MUST map into exactly
    one id. Every definition one id reaches MUST be one fact listing each
    location and kind.
  """
  ensures {
    disjoint_ids "an item id never holds a hash sign, a test id always does"
    round_trip   "every id parses back to itself"
    scip_total   "every SCIP symbol of the two grammars maps into one id"
    one_fact     "definitions of one id merge into one fact listing every location"
  }
  verify unit "an item id never holds a hash sign, a test id always does"
  verify unit "every id parses back to itself"
  verify unit "a SCIP symbol of rust-analyzer or scip-typescript maps into exactly one id"
  verify unit "definitions of one id merge into one fact listing every location and kind"
}
