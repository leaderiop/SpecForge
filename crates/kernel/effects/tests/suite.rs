//! The effect plane's tests: the wire's canonical digest and class strings, the limits, `Contained`, and each fake's
//! contract.
#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

mod contained;
mod digest;
mod fakes;
mod limits;
