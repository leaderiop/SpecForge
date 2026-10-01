//! `specforge explain <CODE>`: print a catalogued diagnostic code
//! ([`specforge_diagnostics`]).

use specforge_diagnostics::{CodeEntry, WRAP_WIDTH, lookup, retired, wrap};

/// Print the explanation of a diagnostic code.
pub fn run(code: &str) -> i32 {
    if let Some(replacement) = retired(code) {
        let old = code.to_uppercase();
        match replacement.and_then(lookup) {
            Some(entry) => {
                println!("{old} is retired; it was renumbered to {}.\n", entry.code);
                print_entry(entry);
            }
            None => println!("{old} is retired and no longer emitted."),
        }
        return 0;
    }
    match lookup(code) {
        Some(entry) => {
            print_entry(entry);
            0
        }
        None => {
            eprintln!("unknown diagnostic code: {code}");
            eprintln!(
                "hint: codes follow the pattern E### (error), W### (warning), I### (info), A### (analyze finding); \
                 registry client codes are R### and R-<AREA>-###"
            );
            eprintln!(
                "hint: E900-E998, W900-W998 and I900-I998 are reserved for third-party extensions; \
                 see the extension's own documentation"
            );
            1
        }
    }
}

fn print_entry(entry: &CodeEntry) {
    println!("\x1b[1m{}\x1b[0m: {}\n", entry.code, entry.title);
    println!("{}", wrap(entry.explanation, WRAP_WIDTH));
    println!("\nOwner: {}", entry.owner);
    println!("Level: {}", entry.level.describe());
}
