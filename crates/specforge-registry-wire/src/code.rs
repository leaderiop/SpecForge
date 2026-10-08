//! The codes a registry's error answers carry (`{"error": {"code": …}}`).

/// Nothing is published under the name or version asked for.
pub const NOT_FOUND: &str = "NOT_FOUND";
/// The bearer token is missing, unknown, expired or revoked.
pub const UNAUTHORIZED: &str = "UNAUTHORIZED";
/// The token is valid but may not do this (a scope it does not cover).
pub const FORBIDDEN: &str = "FORBIDDEN";
/// The endpoint needs an admin token.
pub const ADMIN_REQUIRED: &str = "ADMIN_REQUIRED";
/// The scope was claimed by another publisher.
pub const SCOPE_OWNED: &str = "SCOPE_OWNED";
/// Too many publishes inside the window; the answer carries `Retry-After`.
pub const RATE_LIMITED: &str = "RATE_LIMITED";
/// The request is malformed (a missing form field).
pub const BAD_REQUEST: &str = "BAD_REQUEST";
/// The upload names no scoped package.
pub const INVALID_NAME: &str = "INVALID_NAME";
/// The upload names no SemVer version.
pub const INVALID_VERSION: &str = "INVALID_VERSION";
/// A publish without a signature.
pub const UNSIGNED_PACKAGE: &str = "UNSIGNED_PACKAGE";
/// The uploaded binary is not a Wasm module.
pub const INVALID_WASM: &str = "INVALID_WASM";
/// The uploaded manifest is not an extension declaration.
pub const INVALID_MANIFEST: &str = "INVALID_MANIFEST";
/// The manifest names another package or version than the upload path.
pub const NAME_MISMATCH: &str = "NAME_MISMATCH";
/// The version is already published.
pub const DUPLICATE_VERSION: &str = "DUPLICATE_VERSION";
/// The stored binary does not hash to its recorded digest.
pub const INTEGRITY_VIOLATION: &str = "INTEGRITY_VIOLATION";
/// A database query failed.
pub const DB_ERROR: &str = "DB_ERROR";
/// A database task did not finish.
pub const DB_TASK: &str = "DB_TASK";
/// A token check did not finish.
pub const AUTH_TASK: &str = "AUTH_TASK";
/// An integrity check did not finish.
pub const INTEGRITY_TASK: &str = "INTEGRITY_TASK";
/// A storage write or commit failed.
pub const STORAGE_ERROR: &str = "STORAGE_ERROR";
/// A storage read did not finish.
pub const STORAGE_TASK: &str = "STORAGE_TASK";
