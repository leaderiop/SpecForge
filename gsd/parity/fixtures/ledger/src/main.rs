//! `ledger add <cents> <memo>` prints the store line of a new entry; `ledger total` reads a store on stdin and prints
//! its total.

use std::io::Read;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["add", cents, memo] => {
            let Ok(cents) = cents.parse() else {
                eprintln!("ledger: not cents: {cents}");
                return ExitCode::from(2);
            };
            let mut entries = Vec::new();
            if let Err(why) = ledger::add(&mut entries, cents, memo) {
                eprintln!("ledger: {why}");
                return ExitCode::from(2);
            }
            println!("{}", ledger::line(&entries[0]));
        }
        ["total"] => {
            let mut text = String::new();
            if std::io::stdin().read_to_string(&mut text).is_err() {
                eprintln!("ledger: stdin is not text");
                return ExitCode::from(2);
            }
            match ledger::parse(&text) {
                Ok(entries) => println!("{}", ledger::column(ledger::total(&entries))),
                Err(why) => {
                    eprintln!("ledger: {why}");
                    return ExitCode::from(2);
                }
            }
        }
        _ => {
            eprintln!("usage: ledger add <cents> <memo> | ledger total < store");
            return ExitCode::from(2);
        }
    }
    ExitCode::SUCCESS
}
