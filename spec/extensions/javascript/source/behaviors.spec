// @javascript/source behaviors

use "extensions/code/core/features"
use "types/wasm"

behavior js_declare_source_collector "Declare the javascript-source Collector" {
  features [js_item_facts, cc_code_facts]
  category query
  types    [CollectorDescriptor]
  contract """
    @javascript/source MUST declare one collector, `javascript-source`,
    selected by a package.json at the project root, with no command and no
    report: it reads every package.json with its text and lists the
    project's files of each source extension (.ts .mts .cts .tsx .js .jsx
    .mjs .cjs, one read each, without text), and answers the item and
    package kinds. An fs read capability on the project root, for collect
    calls only, backs its syntax import. It requires @code/core ^1.1.
  """
  ensures {
    detected_by_package_json "a package.json at the project root selects javascript-source"
    no_command               "it runs nothing and asks no consent"
    read_only                "its one capability reads the project root during collect"
  }
  verify unit "javascript-source declares its collector, its inputs, its capability and its peer"
}

behavior js_select_source_files "Select and Parse the Source Files" {
  features [js_item_facts]
  category computation
  contract """
    javascript-source MUST read a listed file only when a named package
    owns it, it is not a test file (a name matching *.test.*, *.spec.*,
    *-test.*, *_test.*, test-*.* or test.*, or a directory segment test,
    tests, __tests__ or __mocks__) and it is not generated output (a
    *.min.js, *.min.mjs or *.min.cjs, or a directory segment coverage,
    build, out, .nuxt, .svelte-kit, .turbo, .cache or storybook-static).
    The kept files of one grammar MUST be parsed in one request, the
    typescript grammar for .ts .mts .cts and tsx for the rest, the files an
    answer's cap left unparsed asked again. A listing the host truncated
    is replaced by a parse of its glob whose answer the same rules filter;
    a walk that stopped short, and a kept file the reader could not parse
    for a reason other than being gone or withheld, are the collector's
    error, never a file dropped silently.
  """
  ensures {
    by_extension "each file is read by the grammar its extension names"
    no_tests     "a test file is never parsed"
    complete     "no kept file is dropped for the answer's cap or the listing's"
  }
  verify unit "a file is read by the grammar its extension names"
  verify unit "a test file is never parsed"
  verify unit "generated output is never parsed"
  verify unit "a file under no named package is not read"
  verify unit "a truncated listing parses its glob and filters the answer"
  verify unit "the files an answer's cap left unparsed are asked again"
}

behavior js_mint_item_facts "Mint Item and Package Facts from Definitions" {
  features [js_item_facts, cc_code_facts]
  category computation
  types    [Fact]
  contract """
    javascript-source MUST merge the matches of one defining node and mint
    it as an item with specforge-code-facts' item_facts when it is at item
    scope: every definition around it a namespace, or, for a method or a
    field, the nearest a class or an interface. Its id is
    js:<package>/<file>::<path>, the path the names around it then its own
    (a nested namespace A.B is two names), its file the path under the
    package's directory, its place the defining node's span. A function-
    or class-valued variable is a function or a class. An unnamed default
    export is the item default. A declaration inside a function, a block
    or a declare module or declare global block MUST NOT be minted, nor a
    member with no class or interface around it. Definitions of one path
    are one item listing every place and kind. Every named package MUST be
    a pkg:npm/<name> package fact with its version, engines, directory and
    the project's own packages it depends on.
  """
  ensures {
    item_scope   "only a declaration an importer could name is an item"
    one_per_path "one item per path, every place listed"
    packages     "every item's package is a package fact"
  }
  verify unit "items are named after their package and file"
  verify unit "a member is named by its class and a namespace member by its namespace"
  verify unit "a local declaration is not an item"
  verify unit "a declare module block mints no item"
  verify unit "an unnamed default export is the item default"
  verify unit "function-valued variables are functions"
  verify unit "definitions of one name are one item with every place"
  verify unit "each named package is a package fact its items point at"
}

behavior js_mark_item_exports "Mark Exports, Visibility, Docs and Signatures" {
  features [js_item_facts, cc_code_facts]
  category computation
  types    [Fact]
  contract """
    javascript-source MUST mark an item public when an importer reaches
    it: a top-level item written with export, assigned to exports.x or
    module.exports.x, or named by an export list without from, by export
    default <name> or by module.exports = { … }; a namespace member
    exported from a public namespace; a member of a public class or
    interface that is neither #private nor private. Every other item is
    private. A name export default names gains the attr default. Its doc
    MUST be the /** */ comment that ends on its first line or the line
    above, the markers and each line's leading * dropped; its signature
    the declared types, parameters, type parameters and heritage the query
    captures, never a value, which item_facts digests; its attrs the
    modifier tokens written on it (export default declare abstract static
    async get set readonly override const let var * exports and the
    accessibility words).
  """
  ensures {
    reachable "public means an importer reaches the item"
    no_values "a signature never holds a value"
    jsdoc     "a JSDoc comment documents the item directly below it"
  }
  verify unit "an exported declaration is public and a local one private"
  verify unit "an export list and module.exports make an item public"
  verify unit "export default of a name marks it default"
  verify unit "a private member of a public class is private"
  verify unit "a member of a private class is private"
  verify unit "a JSDoc comment is the doc of the item below it"
  verify unit "a signature holds types and never a value"
  verify unit "modifiers are attrs"
}
