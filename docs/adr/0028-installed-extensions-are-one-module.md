# Installed extensions are one module

**Status:** accepted (2026-10-07)

How a `specforge.json` entry becomes a loaded extension, and how `add`, `update` and `remove` change
what is installed, had no owner. `.specforge/extensions` was derived in four places; each compile read
`specforge.json` and `specforge.lock` twice; the load policy (lock entry, hash pin, `.wasm` file entry
rules) ran inside the wasmtime adapter and parked its results on the `WasmRuntime` port
(`load_failure`, `file_entry_extension`), so it could only be tested with wasmtime; the hash and the
compile read the file separately. `update` put back what it changed, `add` and `remove` did not: a
failed add left a binary its lock refused, a failed remove a lock entry without a binary, and `add`
replaced a lock it could not read with one holding only the new entry. Doctor reported a changed
binary twice, with two remedies, one of which `add` answered "already installed".

## Decision

- **D1. One crate, `specforge-installed`.** It owns the layout (`.specforge/extensions/<name>/extension.wasm`,
  `specforge.lock`), the lock, the pin, the extension load and changes to what is installed.
  `specforge-wasm` keeps the runtime port, extension calls and the declaration loader;
  `specforge-component` is the wasmtime adapter and the builtin blobs. The crate takes builtins as a
  value and the engine as `&dyn WasmRuntime`, so it depends on neither.
- **D2. `Installed` is a project's installed extensions, its lock read once.** The environment holds
  it (`Environment::installed`); `add` and `update`, which run without a compile, read it with
  `Installed::at`. Nothing else derives an installed path.
- **D3. The environment loads its extensions through `Installed::load`.** Every environment, with
  any runtime, loads through the same policy: a builtin from its bytes, an installed extension from
  its module when its SHA-256 is the pinned one (E070 otherwise, W149 when none is pinned) and it
  declares the extension its entry names, a `.wasm` file entry from its file under the name it
  declares. The bytes hashed are the bytes compiled. Each entry's outcome is a typed `LoadFailure`
  with one diagnostic; the runtime keeps no load state, and the port is `load(name, bytes)`, `rename`,
  `unload`, `call_export`, `apply_limits`. Tests that serve an extension in process install it on disk.
- **D4. One change, all or nothing.** `Installed::change()` stages installs and uninstalls;
  `commit_with` places or moves aside each module (renames under `.specforge/extensions/.staging`),
  writes the lock once, then runs the `specforge.json` edit, and on any failure puts back every file.
  `add`, `update` and `remove` use it. A failed change reports what it could not put back (normally
  nothing) as its error's writes (ADR 0022). A change over an unreadable lock is refused (E033).
- **D5. "Installed" means verified.** `add` treats an extension as already installed only when its
  module is the pinned one, so the reinstall doctor and E070 name repairs it. Doctor reports a
  missing or changed binary once, with that command.
- **D6. Codes.** E070 is a binary that is not the one its lock entry pins; E033 keeps the lock
  file's own problems; W149 is an entry that pins no hash; W119, never reported, is retired.

## Consequences

- A failed `add` or `remove` writes nothing; MCP's `files_written` and the CLI's `wrote:` lines for it
  are empty unless the rollback itself failed.
- `check` reports an unreadable lock once (E033) when an enabled installed extension needs it, before
  the E028 of each such extension; a project that enables none still does not report it.
- A tampered or replaced binary is E070 everywhere it was E033.
- Lock files are byte-identical; the lock entry's source is typed in memory only (`LockSource`).
- `specforge-installed` is a path dependency, not a workspace dependency: the root manifest's
  `[workspace.dependencies]` table is an input of the builtin blobs.

**What would reopen it:** two processes changing one project's extensions at once (the invariant's
"concurrent install and uninstall are serialized" is not enforced: an advisory lock on the staging
directory would be the place), or a crash between moving a module aside and placing the new one
needing automatic recovery (today the next change sweeps the staging directory and doctor reports
the missing binary).
