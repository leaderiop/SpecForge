//! The options `specforge` adds to every `specforge gsd <command>` unless the command declares an arg of the name:
//! `--path` and `--format`, `--host` (ADR 0318), `--effects` (ADR 0337 D4), `--pick` and its shapes (ADR 0325 D4,
//! D8). A writing command also takes `--dry-run`. The ledger's invocation check (`port_tools`) skips them; the
//! black-box `cli::cmd_surface::the_host_options_are_the_binarys` holds the list to the built binary's `--help`.

/// `(long, takes a value)`, in the order `--help` lists them.
pub const HOST_OPTIONS: [(&str, bool); 9] = [
    ("path", true),
    ("format", true),
    ("host", true),
    ("effects", true),
    ("pick", true),
    ("count", false),
    ("first", true),
    ("last", true),
    ("join", true),
];
