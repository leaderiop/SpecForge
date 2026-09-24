use crate::db::Database;
use crate::rate::RateLimiter;
use crate::storage::LocalStorage;

pub struct AppState {
    pub database: Database,
    pub storage: LocalStorage,
    /// Fixed-window rate limiter shared by all keyed checks.
    pub rate_limiter: RateLimiter,
    /// Publish requests allowed per token per window.
    pub publish_limit_per_token: u32,
    /// Publish requests allowed per client IP per window.
    pub publish_limit_per_ip: u32,
}
