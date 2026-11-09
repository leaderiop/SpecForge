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
- **Test files**: a name holding `.test.`, `.spec.`, `-test.` or `_test.`, or
  starting `test-` or `test.`, and anything under a `test`, `tests`,
  `__tests__` or `__mocks__` directory. A runner pack records the tests; their
  helpers are no API.
- **Generated output**: a `.min.js`, `.min.mjs` or `.min.cjs` file, and
  anything under `coverage`, `build`, `out`, `.nuxt`, `.svelte-kit`, `.turbo`,
  `.cache` or `storybook-static`. The host's walk never enters `dist`,
  `node_modules`, `.next`, `vendor` or `.git`.

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
  `export { a as b }` keeps `::a`, only an unnamed default export is
  `::default`. A string member name loses its quotes.
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

## 6. What an item knows

- **`visibility`**: `public` when an importer reaches it, else `private`. A
  top-level item is public when exported: `export`, `exports.x =`, an export
  list or `module.exports = { … }` naming it. A namespace member is public
  when exported from a public namespace; a class or interface member when its
  class is public and it is neither `#private` nor `private`.
- **`attrs`**: the modifier words written on it (`export`, `default`,
  `declare`, `async`, `static`, `private`, …).
- **`doc`**: the `/** */` comment ending on its first line or the line above,
  without its markers.
- **`signature`**: its declared parameters, types and heritage, never a value.
- **`package`**: its package fact, `pkg:npm/<name>`.

## 7. The package facts

One `package` fact per named package: its `version`, its `engines`
(`node >=20`), its directory as `path` (empty at the project root) and, in
`depends_on`, the project's own packages its `dependencies`,
`devDependencies`, `peerDependencies` or `optionalDependencies` name.
Registry dependencies are left out: they belong to a later
`@javascript/package`, which reads the lockfile and takes over these facts.
Two `package.json` of one name are one package, the shallowest's.

## Troubleshooting

| What you see | Why, and the fix |
|---|---|
| `<file>: the host's reader did not parse it (<why>)` | The host withheld the file (too large, not UTF-8) or its parse ran out of time. The pack reads every kept file or none: move a generated file under a skipped directory or rename it `.min.js`. |
| `<file>: its matches alone pass the host's answer cap` | One file declares more than an answer holds; it is almost always generated. |
| `**/*.<ext>: the project holds more files than the host walks` | The project holds over 20,000 entries the walk visits. |
| No item for a file you expected | It is a test file, generated output, or under no named package (section 3). |
