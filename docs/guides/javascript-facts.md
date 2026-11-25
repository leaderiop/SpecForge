# Recording a JavaScript project's packages and items as facts

This guide makes a JavaScript or TypeScript project's packages and source into
facts the spec can cite: each named package becomes a `package` entity, and
each function, class, type, namespace and value its files declare an `item`.
It takes two extensions, and a third for the tests:

- **`@code/core`** declares the fact kinds and checks every reference a spec
  writes to them ([ADR 0347](../adr/0347-item-and-package-facts-and-ids-a-scip-symbol-maps-into.md)).
- **`@javascript/source`** reads every `package.json` with `serde_json` and has
  SpecForge's own reader parse the source files. It runs nothing: no `node`,
  no TypeScript compiler, nothing to install.
- **`@javascript/node-test`** records the `node --test` tests
  ([guide](node-test-facts.md)). Both packs name a file's package by one rule,
  so the tests and the items of a package name it alike.

## 1. Enable the extensions

```bash
specforge add @javascript/source   # also enables @code/core
```

## 2. Collect

```bash
specforge collect --runner javascript-source
```

A `package.json` at the project root selects the pack, so a plain
`specforge collect` runs it beside every other collector it detects. It asks
for no approval: it reads the project's files and runs no command.

## 3. What is read, and what is not

The pack reads every file ending in `.ts`, `.mts`, `.cts` (a `.d.ts` too)
with the TypeScript grammar, and every `.tsx`, `.js`, `.jsx`, `.mjs` and
`.cjs` with the TSX grammar, which reads JavaScript and JSX as well. It skips:

- **A file under no named package.** A file's package is the nearest
  `package.json` with a `name` at or above it. A stray script outside every
  named package is outside the pack, as `node-test` refuses a test there.
- **Test files, but for their test calls**: a name holding `.test.`,
  `.spec.`, `-test.` or `_test.`, or starting `test-` or `test.`, and anything
  under a `test`, `tests`, `__tests__` or `__mocks__` directory. A second
  query reads for its statically named test calls only, each an item
  (section 5), a test file `node --test` runs by default, since node-test
  names the tests: `*.test.*`, `*-test.*`, `*_test.*`, `test-*.*`, `test.*`
  and anything under a `test` directory of the project, in `.js`, `.cjs`,
  `.mjs`, `.ts`, `.mts` or `.cts` (node 23.6 and later run TypeScript). A
  test file of a package this pack does not read (a fixture under `test/`)
  gives no item, as node-test names its tests after that package. A test
  file's helpers are no API and no item.
- **Generated output**: a `.min.js`, `.min.mjs` or `.min.cjs` file, and
  anything under `coverage`, `build`, `out`, `.nuxt`, `.svelte-kit`, `.turbo`,
  `.cache` or `storybook-static`. The host's walk never enters `dist`,
  `node_modules`, `.next`, `vendor` or `.git`.

The test and generated-output rules judge a file's path inside its package.
A `package.json` is judged the same way, on its path inside the package
above it, or in the project when there is none: one under a test or
generated-output directory (a fixture under `tests/`, a copy under `build/`)
is no package of this pack, so no package fact, and its files are unread.

Compiled output committed under another name (`lib/`) is read: its
definitions become items. There is no per-project exclusion yet.

## 4. Which declarations are items

A declaration is an item at **item scope**: the top level of a file or the
body of a namespace. A method or a field is an item when the class or
interface around it is. Nothing declared inside a function, a block, a
`declare module "x"` or a `declare global` is an item, nor is a class
expression handed to a call.

| Written | Item | `kind` |
|---|---|---|
| `function add(a, b) {}`, `const add = (a, b) => …` | `::add` | `function` |
| `class Board { cells = []; place() {} }` | `::Board`, `::Board.cells`, `::Board.place` | `class`, `field`, `method` |
| `interface Shape { area(): number }` | `::Shape`, `::Shape.area` | `interface`, `method` |
| `type Id = string; const Id = …` | `::Id`, one item | `type`, `const` |
| `namespace Geo { export function distance() {} }` | `::Geo`, `::Geo.distance` | `namespace`, `function` |
| `enum Color {}`, `let count = 0` | `::Color`, `::count` | `enum`, `variable` |
| `exports.build = …`, `module.exports = { clean }` | `::build`, `::clean` | per its value |

- Overloads and the body of one function are one item listing every place.
- An id names the declared name: `export default function main` is `::main`,
  `export { a as b }` keeps `::a` (`as default` adds the attr `default`), only
  an unnamed default export is `::default`. A string member name is its value (`'a\'b'() {}` is `a'b`, a
  surrogate pair written `\uD83D\uDE00` is one character), and `namespace A.B` is the two names `A` and `B`,
  up to six of them: a dotted name of seven or more is no item. The reader's query has no pattern that reaches
  a segment at any depth, and the pack never splits the name's text itself, which a space or a comment between
  segments would fool.
- CommonJS's `module.exports = <function>`, TypeScript's `export =` and a
  re-export `export … from` define no item here.

## 5. The ids

| Fact | Id | Example |
|---|---|---|
| `package` | `pkg:npm/<name>`, a scope's `@` written `%40` | `pkg:npm/%40acme/core` |
| `item` | `js:<package>/<file>::<name>[.<member>]` | `js:@acme/core/src/index.ts::Board.place` |
| `test` | `js:<package>/<file>#<suite > … > name>` | `js:@acme/core/test/board.test.ts#adds` |

`<file>` is the path under the package's directory; the fact's `file` field
is its path from the project root. An item id never holds `#`, a test id
always does, so an item and a test of one file cannot clash.

A test call in a test file is a `function` item with the attribute `test`
([ADR 0393](../adr/0393-x2-is-a-floor-every-real-test-reaches-and-only-tests-count.md)).
Its id is `js:<package>/<file>::<suite>.<name>`, the names of the calls
around it outermost first, then its own. A call node reports as a test case
(below) also carries `test_id`,
`js:<package>/<file>#<suite > … > name>`, the id `@javascript/node-test`
gives the same test, so X2 counts it:

```js
// package.json: { "name": "taskboard" }   test/tasks.test.js:
test('a task is added with the next id', () => { /* ... */ })
// item js:taskboard/test/tasks.test.js::a task is added with the next id
// test_id js:taskboard/test/tasks.test.js#a task is added with the next id
```

- A call counts when its name is static and not empty: a string, or a
  template with no substitution. It is `describe` or `suite`, `test` or `it`
  with a function argument, one of those with `.skip`, `.todo` or `.only`, or
  a subtest `t.test(name, fn)` inside a test.
- node's JUnit report writes a test or a suite as a `<testsuite>` when a
  test, suite or subtest ran inside it, else as a `<testcase>`. So a call
  carries a test id when it has `.skip`, or when no `test`, `it`, `describe`,
  `suite` or subtest call sits inside it: an empty `describe` and one with no
  function are test cases. A call holding one, even one that is no item (a
  subtest named by a variable or given a function reference), carries none.
- node never runs a skipped call's function, so nothing inside a `.skip`
  call, or a call whose options have a `skip` key (`{ skip: true }`), is an
  item. Such a call with options carries a test id only when nothing sits
  inside it. A `.todo` call runs its function: what it holds is read.
- A dynamic name (`test(name, …)`, `.each`, `.concurrent`) or an empty one
  (node names it `<anonymous>` or after its function) mints nothing, nor does
  anything inside that call, and raises nothing.
- Subtests a helper registers out of the call's sight (`await check(t)`) are
  not seen: the test keeps its test id though node reports it as a suite.

Two `package.json` may give one name (an example app beside the real one):
the package in the shallowest directory (then the lexically first) keeps
`pkg:npm/<name>`, every other is `pkg:npm/<name>#<dir>`, and its items and
tests name it `<name>@<dir>` with the directory's `/` written `%2F`:
`js:app@examples%2Fapp/src/a.js::x`.

## 6. What an item knows

- **`visibility`**: `public` when an importer reaches it, else `private`. A
  top-level item is public when exported: `export`, `exports.x =`, an export
  list or `module.exports = { … }` naming it. A namespace member is public
  when exported from a public namespace; a class or interface member when its
  class is public and it is neither `#private` nor `private`.
- **`attrs`**: the modifier words written on it (`export`, `default`,
  `declare`, `async`, `static`, `private`, …).
- **`doc`**: the `/** */` comment ending on its first line or the line above,
  without its markers. A decorated class or member starts at its first
  decorator, so `/** … */` above `@dec class X`, above `@dec export class X`
  and above `@dec method() {}` in a class is its doc.
- **`signature`**: a digest of its declared parameters, types and heritage,
  never a value. A change of layout is no change; a change inside a string,
  a template or a regular expression is (`'a  b'` is another type than
  `'a b'`).
- **`package`**: its package fact, `pkg:npm/<name>`.

## 7. The package facts

One `package` fact per package the pack reads: its `version`, its `engines`
(`node >=20`), its directory as `path` (empty at the project root) and, in
`depends_on`, the project's own packages its `dependencies`,
`devDependencies`, `peerDependencies` or `optionalDependencies` name.
Registry dependencies are left out: they belong to a later
`@javascript/package`, which reads the lockfile and takes over these facts.
A second package of one name is its own fact, `pkg:npm/<name>#<dir>`; a
dependency on that name is the shallowest the pack reads.

## Troubleshooting

| What you see | Why, and the fix |
|---|---|
| `<file>: the host's reader did not parse it (<why>)` | The host could not read the file (too large, not UTF-8), refused it (`denied`: a name the fs port withholds, a link out of the project, a file with a second name) or its parse ran out of time. The pack reads every kept file or none: move a generated file under a skipped directory or rename it `.min.js`. |
| `<file>: its matches alone pass the host's answer cap` | One file declares more than an answer holds; it is almost always generated. |
| `**/*.<ext>: the project holds more files than the host walks` | The project holds over 20,000 entries the walk visits: the files past them cannot be asked, whatever else the answer holds. |
| No item for a file you expected | It is generated output or under no named package (section 3). In a test file only the statically named test calls are items: a helper is none, and a dynamically named test has none (section 5). |
