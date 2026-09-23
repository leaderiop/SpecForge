//! SpecForge extension registry server library.
//!
//! Exposed as a library so integration tests can drive the real router,
//! database, and storage against an in-process server (spec #21, seam 1).
pub mod auth;
pub mod db;
pub mod handlers;
pub mod state;
pub mod storage;
