# Case Study: Redis Lua Scripting — Sandboxing, Security, and Runtime Limits

Peer research for the SpecForge plugin-runtime decision (KEEP_WASM vs embed Lua/Python/TypeScript vs MULTI). Redis is the most heavily deployed embedded-language plugin system in existence: a single-threaded network daemon that runs untrusted user scripts in-process since 2012. Its decade-long incident history is the closest thing we have to a longitudinal field trial of an in-process scripting sandbox.

## 1. How Redis sandboxes Lua

Redis embeds a **Lua 5.1 interpreter in-process** (its own patched fork, not LuaJIT) and runs scripts synchronously on the main server thread ([programmability docs](https://redis.io/docs/latest/develop/programmability/)). The sandbox is a **soft, interpreter-level sandbox** — no OS process, thread, or memory isolation:

- **No I/O, no host access.** Scripts "should never try to access the Redis server's underlying host systems... the file system, network, or... any other system call other than those supported by the API" ([lua-api](https://redis.io/docs/latest/develop/programmability/lua-api/)).
- **Globals blocked.** Global variable/function declarations are rejected via a protection metatable; state must live in the keyspace. Redis's own docs concede the model: *"Using Lua's debugging functionality or other approaches such as altering the meta table used for implementing the globals' protection to circumvent the sandbox isn't hard. However, it is difficult to circumvent the protection by accident."* ([lua-api](https://redis.io/docs/latest/develop/programmability/lua-api/))
- **`require` disabled.** Only bundled libraries are callable: `cjson`, `cmsgpack`, `struct`, `bit`, and a stripped `os` (clock/time only) ([lua-api, runtime libraries](https://redis.io/docs/latest/develop/programmability/lua-api/)).
- **Narrow host bridge.** Everything the script can do to the database goes through `redis.call()` / `redis.pcall()` plus reply helpers (`redis.error_reply`, `redis.status_reply`, `redis.sha1hex`, `redis.log`, `redis.set_repl`, `redis.acl_check_cmd`) ([lua-api, redis object](https://redis.io/docs/latest/develop/programmability/lua-api/)).
- **Determinism constraints.** For replication safety, scripts historically had to be deterministic; `math.random` is re-seeded per run and non-deterministic-command-then-write was blocked pre-7.0 ([eval-intro](https://redis.io/docs/latest/develop/programmability/eval-intro/)).

The critical architectural fact: **the sandbox boundary is the interpreter's own feature set.** Any memory-safety bug in the interpreter, its bundled C libraries, or the Redis↔Lua glue is inside the trust boundary by construction.

## 2. Known security incidents

| CVE | Year | Class | CVSS | Outcome |
|---|---|---|---|---|
| [CVE-2022-0543](https://nvd.nist.gov/vuln/detail/CVE-2022-0543) | 2022 | Sandbox escape → RCE | **10.0** (PR:N) | Debian/Ubuntu-specific packaging bug: dynamically-linked builds left `package.loadlib` functional, letting a script re-open `luaopen_io`/`luaopen_os` from `liblua5.1.so` and recover the `io`/`os` libraries for arbitrary command execution ([PoC walkthrough](https://www.ubercomp.com/posts/2022-01-20_redis_on_debian_rce), [DSA-5081](https://www.debian.org/security/2022/dsa-5081)). In CISA's [Known Exploited catalog](https://www.cisa.gov/known-exploited-vulnerabilities-catalog?field_cve=CVE-2022-0543). |
| [CVE-2025-49844](https://nvd.nist.gov/vuln/detail/CVE-2025-49844) ("RediShell") | 2025 | Use-after-free → RCE | **9.9** (CWE-416) | Authenticated user manipulates the Lua **garbage collector** from a crafted script to trigger a UAF in the interpreter glue and achieve RCE. "The problem exists in **all versions** of Redis with Lua scripting"; fixed in 8.2.2 (and backports); official workaround is *disabling scripting entirely* via ACL. NVD also lists **Valkey** as affected ([GHSA-4789-qfc9-5f9q](https://github.com/redis/redis/security/advisories/GHSA-4789-qfc9-5f9q)). |
| [CVE-2024-31449](https://nvd.nist.gov/vuln/detail/CVE-2024-31449) | 2024 | Stack buffer overflow → RCE | 7.0 (CWE-121) | Crafted script overflows a stack buffer in the bundled `bit` library. "Exists in all versions of Redis with Lua scripting"; fixed 6.2.16 / 7.2.6 / 7.4.1, **no known workarounds** short of patching. |
| [CVE-2022-24735](https://nvd.nist.gov/vuln/detail/CVE-2022-24735) | 2022 | Script injection / privilege escalation | 3.9 (CWE-94) | Long-known weaknesses in the sandbox's state-persistence prevention let a low-privilege ACL user inject Lua that executes **with a more privileged user's rights** later. Fixed 6.2.7 / 7.0.0; workaround: ACL-block `EVAL`/`SCRIPT LOAD`. ([PR 10651](https://github.com/redis/redis/pull/10651)) |
| [CVE-2022-24736](https://nvd.nist.gov/vuln/detail/CVE-2022-24736) | 2022 | NULL deref → server crash | 3.3 (CWE-476) | Specially crafted script load crashes redis-server (DoS). Fixed alongside 24735. |

Two patterns matter for SpecForge:

1. **The escapes were never the documented sandbox policy.** They were C-level memory bugs in the interpreter (GC, `bit`), the build/packaging layer (`loadlib`), or the glue — all *inside* the process boundary. Redis's response repeatedly includes "disable EVAL via ACL" as the only complete mitigation, which is an implicit admission that the soft sandbox cannot be made a hard security boundary.
2. **"All versions with Lua scripting"** appears in the 2024 and 2025 advisories: the in-process-interpreter architecture carries the whole accumulated bug surface of the embedded runtime, forever. Docker escape PoCs and mass-scan tooling followed CVE-2022-0543 within weeks (it became a staple of ransomware toolkits, per the KEV listing).

## 3. The EVAL API surface

`EVAL script numkeys [key [key ...]] [arg [arg ...]]` — since 2.6.0 ([EVAL docs](https://redis.io/docs/latest/commands/eval/)):

- **`script`**: full Lua source, compiled and cached by SHA1. **`numkeys` + `key`s** populate the `KEYS` global; remaining args populate `ARGV`. Keys-declared-as-keys is mandatory for cluster routing; key specs mark scripts `RW`/`update` because "we cannot tell how the keys will be used."
- **Family**: `EVALSHA` (run cached by digest, `NOSCRIPT` on miss), `EVAL_RO`/`EVALSHA_RO` (read-only variants, 7.0), `SCRIPT LOAD|EXISTS|FLUSH|KILL|DEBUG`, and since 7.0 the persistent **Functions** API (`FCALL`/`FCALL_RO`, `FUNCTION LOAD|LIST|DELETE|KILL|DUMP|RESTORE`).
- **Script cache** is volatile (never persisted; cleared on restart/failover) and, since 7.4, **LRU-evicts** scripts past a size cap with an `evicted_scripts` INFO metric — a direct response to memory-exhaustion abuse via machine-generated `EVAL` bodies.
- **Capability flags via shebang** (7.0): `#!lua flags=no-writes,allow-oom,allow-stale,allow-cross-slot-keys` declare how a script touches data; read-only scripts gain the ability to run on replicas, always be killable, and never fail on OOM ([programmability](https://redis.io/docs/latest/develop/programmability/)).
- **Return path**: Lua tables/strings/numbers convert to RESP per fixed conversion tables; errors propagate as RESP errors, or are caught via `redis.pcall()` ([lua-api](https://redis.io/docs/latest/develop/programmability/lua-api/)).

## 4. Limits: CPU, memory, time

- **Time**: `busy-reply-threshold` (default **5000 ms**, alias for the old `lua-time-limit` — verified as a rename in [config.c:3237](https://github.com/redis/redis/blob/8.2.2/src/config.c)). Runtime-configurable via `CONFIG SET`, ms precision.
- **CPU**: no per-script CPU quota and no instruction budget beyond the wall-clock hook. One script on the main thread **blocks the entire server** — atomicity is the feature; isolation from the event loop is the cost.
- **Memory**: **no per-script memory quota in OSS Redis**. The Lua heap allocates through the server's allocator, so script memory counts against `used_memory`/`maxmemory`; under `maxmemory`, the script's first memory-growing write aborts it (recoverable via `redis.pcall`), while memory-free first writes (e.g. `DEL`) are allowed to run, possibly pushing past the limit ([eval-intro, low-memory](https://redis.io/docs/latest/develop/programmability/eval-intro/)). Script-cache growth is capped only by the 7.4 LRU eviction.

## 5. What happens when a script hangs

Redis **refuses to kill scripts automatically** — "Doing so would violate the contract... that ensures that scripts are atomic" ([programmability, maximum execution time](https://redis.io/docs/latest/develop/programmability/)). The enforcement is **cooperative, not preemptive**: Redis installs a Lua debug count hook — `lua_sethook(lua, luaMaskCountHook, LUA_MASKCOUNT, 100000)` — which fires every 100,000 VM instructions and checks elapsed wall time ([script_lua.c:1669-1670](https://github.com/redis/redis/blob/unstable/src/script_lua.c)). On threshold breach, `scriptInterrupt()` flips the run context to `SCRIPT_TIMEDOUT` and enters blocked-mode event processing ([script.c:141-170](https://github.com/redis/redis/blob/unstable/src/script.c)):

1. Redis logs the slow script; **the script keeps running**.
2. Other clients' normal commands get **`BUSY` errors**; only `SCRIPT KILL`, `FUNCTION KILL`, and `SHUTDOWN NOSAVE` are accepted.
3. `SCRIPT KILL` works only if the script has performed **zero writes** (killing a read-only script preserves atomicity). The hook then re-arms to `LUA_MASKLINE` and raises an uncatchable Lua error — deliberately hook-every-line so a `pcall` cannot swallow the kill ([script_lua.c:1601-1613](https://github.com/redis/redis/blob/unstable/src/script_lua.c)).
4. If **any write** already happened, the only option is `SHUTDOWN NOSAVE` — the server aborts without persisting, sacrificing in-memory data since the last save to evict the hung script.

So the worst case for a buggy or malicious plugin in Redis is: block the database, wait out the threshold, then either `SCRIPT KILL` (clean, read-only) or an intentional server crash (wrote anything). A `while true do end` loop is killable; a script that wrote once and then hangs is not recoverable without data loss.

## 6. Implications for the SpecForge decision

- **Soft in-process sandboxes accumulate unbounded CVE debt.** Three RCE-class bugs in four years (2022 ×2 counting 0543, 2024, 2025) all exploiting interpreter-internal memory safety, with "disable scripting" as the canonical mitigation. A Rust host embedding mlua inherits C-Lua-5.1 + user-library memory safety as its own trust boundary.
- **Cooperative cancellation means unkillable state.** Redis's instruction-count hook is the best a non-preemptible interpreter can do, and it still cannot terminate a script mid-write — the host must choose between atomicity and liveness. Wasm gives the host both: isolation (linear memory, no ambient I/O) *and* true preemption (Wasmtime epoch/fuel interruption at any point), without an atomicity contract that forces `SHUTDOWN NOSAVE`.
- **Redis's design center — single-threaded atomic scripts — is not SpecForge's.** SpecForge plugins don't need script-atomic-keyspace semantics; they do need untrusted-code containment. Redis is a cautionary tale for the embed-native-interpreter option, not a template to copy.

## Bottom line

**KEEP_WASM**, confidence **5** — Redis proves that even a decade-mature, strictly-allowlisted in-process interpreter sandbox produces recurring RCE-class escapes rooted in interpreter memory safety, and that cooperative timeouts leave host liveness hostage to plugin state — exactly the failure modes Wasmtime's isolated linear memory and preemption eliminate.
