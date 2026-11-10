// @rust/source behaviors

use "extensions/code/core/features"
use "types/wasm"

behavior rs_declare_source_collector "Declare the rust-source Collector" {
  features [rs_item_facts, cc_code_facts]
  category query
  types    [CollectorDescriptor]
  contract """
    @rust/source MUST declare one collector, `rust-source`, selected by a
    Cargo.toml at the project root, with no command and no report: it is
    served the package facts @rust/cargo answers and answers the item kind.
    An fs read capability on the project root, for collect calls only,
    backs its syntax import. It requires @code/core ^1.1 and @rust/cargo
    ^1.0 (ADR 0348 D6).
  """
  ensures {
    detected_by_cargo_toml "a Cargo.toml at the project root selects rust-source"
    reads_packages         "its input is the package facts"
    read_only              "its one capability reads the project root during collect"
  }
  verify unit "rust-source declares its collector, its input, its capability and its peers"
}

behavior rs_resolve_module_tree "Resolve Each Target's Module Tree" {
  features [rs_item_facts]
  category computation
  contract """
    rust-source MUST walk each target's module tree breadth-first from its
    root by rustc's rules: `mod b;` is <dir>/b.rs, else <dir>/b/mod.rs; a
    crate root, a mod.rs and a file a path attribute reaches keep their
    children beside them, any other file in a directory of its stem; an
    inline module adds its own path attribute's directory, else its name
    without r#, to the directory; a path attribute is
    relative to the declaring file's directory, or inside an inline module
    to that module's; every path a cfg_attr names is followed; a file
    already in the target's tree is not walked again. Each round's
    new files MUST be parsed in one request, the files an answer's cap left
    unparsed asked again. A module whose file is absent or outside the
    project keeps its mod item and no items; a file the reader could not
    parse otherwise is the collector's error.
  """
  ensures {
    rustc_rules "a module's file is the one rustc reads"
    one_parse   "a file two targets reach is parsed once"
    complete    "no file is dropped for the answer's cap"
  }
  verify unit "the tree follows mod declarations from each target's root"
  verify unit "a non-mod-rs file keeps its children in a directory of its stem"
  verify unit "a path attribute is relative to the declaring file's directory"
  verify unit "a path inside an inline module is relative to the inline module's directory"
  verify unit "an inline module's directory is its own path, else its name without r#"
  verify unit "a path attribute looping back to a file already in the tree keeps its mod item and ends there"
  verify unit "every path a cfg_attr names is followed and its twins merge into one item"
  verify unit "a file two targets reach mints items in each crate"
  verify unit "a module whose file is absent or outside the project keeps its mod item only"
  verify unit "the files an answer's cap left unparsed are asked again"
}

behavior rs_mint_item_facts "Mint Item Facts from Definitions" {
  features [rs_item_facts, cc_code_facts]
  category computation
  types    [Fact]
  contract """
    rust-source MUST mint every definition at module level, in inline
    modules and in impl and trait blocks as an item with
    specforge-code-facts' item_facts: a function in an impl or trait is a
    method named by its owner, its visibility the captured modifier's, its
    place the definition's span. A definition inside a function body or
    another item's block MUST NOT be minted, nor a variant or a field. An
    attribute or doc comment belongs to the item only when it is in the
    contiguous run of attribute and doc lines directly above it.
  """
  ensures {
    every_item  "each nameable definition is one item"
    owner_named "an impl or trait item is named by its owner"
    no_locals   "no item inside a function body"
    attached    "an attribute belongs to the item directly below it"
  }
  verify unit "an item carries its kind, visibility, signature, doc and location"
  verify unit "impl and trait items are named by their owner"
  verify unit "an item inside a function body or another item's block is not minted"
  verify unit "attributes and doc comments belong to the item directly below them"
  verify integration "a second collect answers the same facts"
  verify integration "a fixture crate's items are minted from its module tree by the host's reader"
  verify integration "a source collect stores exactly the items of the files it reads"
  verify integration "the source guest maps the fixture crate within a counted work budget"
}

behavior rs_mark_tests "Mark Test Functions with Their Runner Id and Entity" {
  features [rs_item_facts, cc_test_facts]
  category computation
  types    [Fact]
  contract """
    A function marked by an attribute whose path is `test`, ends in
    `::test`, or is `specforge_test` or `spec` MUST carry attrs `test` and
    test_id, the id nextest gives it: ItemId::test_id in the crate's
    edition, a reserved word written raw. `#[ignore]` on a test MUST add
    `ignore`. The first `<kind> = "<id>"` of a specforge_test, spec or
    {specforge_test_macros,specforge_test,specforge}::test attribute MUST be
    written to names with Fact::name, at the id's string; another runner's
    attribute names nothing (ADR 0348 D7).
  """
  ensures {
    runner_id "a test item holds the id its runner gives it"
    ignored   "an ignored test is marked"
    entity    "a spec-link attribute names its entity where it is written"
  }
  verify unit "a test function carries the id nextest gives it"
  verify unit "the same test path in three binaries is three ids"
  verify unit "an ignored test is marked ignore"
  verify unit "a spec-link attribute names its entity at the id's string"
  verify unit "a runner attribute whose path ends in ::test marks a test"
  verify unit "a test in an inline module keeps its module path"
}
