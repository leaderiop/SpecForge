# 122 — Kevin Knapp

**Cluster:** C14 — Direct-dependency maintainers (surface infrastructure)
**Roster role:** clap creator; CLI argument-parsing UX
**SpecForge anchors:** crates/specforge-cli/src/main.rs (34-command tree, derive API), clap_complete::generate shell completions (main.rs)

## Why this engineer
Knapp (kbknapp) created clap in 2015 and led it through v3's maturation of the builder/derive duality that specforge-cli's 34 commands use; the shell-completion surface (`clap_complete::generate` for `specforge`) is his project's companion crate. The v4 era's stewardship handover to Ed Page — keeping clap's conventions stable while maintenance scaled — is the working model for how a spec CLI with a large command surface should evolve without breaking scripts.

## References for SpecForge
**Key works**
- [clap-rs/clap](https://github.com/clap-rs/clap) — GitHub, 2015. The parser behind specforge-cli's command tree (workspace pins 4.5 + derive).
- [clap-rs/clap — clap_complete crate](https://github.com/clap-rs/clap/tree/master/clap_complete) — GitHub. Generates the shell completions specforge-cli emits from the same command definition.
- [clap docs on docs.rs](https://docs.rs/clap) — Derive API reference: `#[command]`, subcommands, arg groups, value parsing.

## Study first
1. Derive vs builder trade-offs for a 34-command tree in main.rs
2. Subcommand-required patterns and global args (config/registry flags across commands)
3. Completion generation parity across bash/zsh/fish from one Command definition
