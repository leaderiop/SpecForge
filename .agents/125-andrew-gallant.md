# 125 — Andrew Gallant

**Cluster:** C14 — Direct-dependency maintainers (surface infrastructure)
**Roster role:** regex/ripgrep/walkdir author; automata-based text search
**SpecForge anchors:** regex constraint compilation in crates/specforge-registry/src/compilation/validation_engine.rs (regex 1.12); file discovery via walkdir (2.5) in crates/specforge-resolver/src/resolve.rs, crates/specforge-lsp/src/backend.rs, crates/specforge-formatter/src/discover.rs

## Why this engineer
Two SpecForge stages run on Gallant's (BurntSushi's) crates: declarative regex field constraints in the registry validation_engine compile through the regex crate — RE2 lineage, linear-time engines, literal optimizations — and .spec file discovery walks via his walkdir. His write-ups on why no-backtracking regexes are a correctness guarantee for user-supplied patterns, and how ripgrep's walker stack parallelizes discovery, apply directly to constraint-compilation safety and resolver cold-start performance.

## References for SpecForge
**Key works**
- [rust-lang/regex](https://github.com/rust-lang/regex) — GitHub, 2014. The engine compiling SpecForge's constraint regexes; RE2-derived linear-time guarantees.
- [BurntSushi/ripgrep](https://github.com/BurntSushi/ripgrep) — GitHub, 2016. The most-used Rust search tool; its walker/ignore design is the reference for file discovery at scale.
- [BurntSushi/walkdir](https://github.com/BurntSushi/walkdir) — GitHub, 2015. The directory walker SpecForge uses for workspace traversal.
- [ripgrep is faster than {grep, ag, git grep, ucg, pt, sift}](https://blog.burntsushi.net/ripgrep/) — blog.burntsushi.net, 2016. Measurement discipline plus the literal-optimization story behind fast regex search.
- blog.burntsushi.net — ongoing essays on regex internals (NFA/DFA engines, Unicode, benchmarks).

## Study first
1. regex crate syntax limits (no look-around) vs validation_engine constraint expressiveness
2. Lazy-DFA and literal-prefix optimizations: cost model for compiled constraints
3. walkdir knobs (min_depth, follow_links, same_file_system) for resolver/lsp discovery
