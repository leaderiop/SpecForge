# 029 — Esteban Küber

**Cluster:** C4 — Incremental compilation, LSP, diagnostics UX
**Roster role:** rustc diagnostics lead
**SpecForge anchors:** Diagnostic model + ariadne rendering, E/W/I codes, did-you-mean (crates/specforge-emitter/src/diagnostic_fmt.rs, crates/specforge-validator/src/file_ref.rs)

## Why this engineer
Küber shaped modern rustc errors into a structured discipline: a rich Diagnostic model (primary span + labels + notes), stable error codes (E0382…) backed by long-form explanations, and machine-applicable suggestions including did-you-mean renames. SpecForge's diagnostic stack — E/W/I codes, ariadne-rendered spans, `did you mean '{}'?` in the validator — is a small-scale edition of the same program, and his model is the checklist for what each diagnostic still lacks: labeled secondary spans, structured suggestion applicability, and code-addressable explanations.

## References for SpecForge
**Key works**
- [Diagnostics chapter — Rustc Dev Guide](https://rustc-dev-guide.rust-lang.org/diagnostics.html) — rust-lang, ongoing. The documented DiagnosticBuilder/snippet/suggestion model; SpecForge's diagnostic_fmt.rs should be auditable against it.
- [Rust error codes index](https://doc.rust-lang.org/error_codes/error-index.html) — rust-lang, ongoing. Each code links to a full explanation page — the pattern for SpecForge E/W/I code documentation in schema/.
- [rust-lang/rust (rustc_errors)](https://github.com/rust-lang/rust) — GitHub, 2010. The reference implementation of structured diagnostics at production scale.
- Compiling to understand: how the Rust compiler reads your mind — Rust Latam, 2019. Küber's talk on diagnostics as the compiler's primary communication channel.

## Study first
1. rustc_errors Diagnostic model: spans, labels, suggestions, applicability levels
2. Error-code long explanations as user education, not error strings
3. JSON diagnostic output for editor-side remapping
