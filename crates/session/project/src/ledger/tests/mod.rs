mod chain;
mod check_run;
mod classes;
mod inputs;
mod keys;
mod links;
mod reads;
mod stamp_order;
mod way;

/// The clock the ledger's tests read (an injected `now`, ADR 0337 D5): a record expiring at or before it is expired.
const NOW: specforge_effects::UnixMillis = specforge_effects::UnixMillis(1_000_000_000_000);
