//! Every integration test of the crate in one binary: each `tests/*.rs` linked wasmtime and the crate on its
//! own, and the links were most of a CI build. The files stay where they are, so their paths do not change.

#[path = "configured.rs"]
mod configured;
#[path = "contract.rs"]
mod contract;
#[path = "fetch.rs"]
mod fetch;
#[path = "publish.rs"]
mod publish;
#[path = "reply_values.rs"]
mod reply_values;
#[path = "requests.rs"]
mod requests;
#[path = "search.rs"]
mod search;
#[path = "user.rs"]
mod user;
