use sha2::{Digest, Sha256};

use crate::db::{Database, TokenRecord};

/// Create a token. `expires_in_days` = `Some(0)` expires immediately;
/// `None` never expires (the `--no-expiry` escape). Default policy is 90 days.
pub fn create_token(
    db: &Database,
    scope: Option<&str>,
    label: &str,
    expires_in_days: Option<u64>,
    admin: bool,
) -> String {
    let raw_token = generate_raw_token();
    let hash = hash_token(&raw_token);
    let expires_at = expires_in_days
        .map(|days| (chrono::Utc::now() + chrono::Duration::days(days as i64)).to_rfc3339());
    db.insert_token(&hash, scope, label, expires_at.as_deref(), admin)
        .expect("failed to store token");
    raw_token
}

/// Whether the record grants admin (token administration) rights.
pub fn is_admin(record: &TokenRecord) -> bool {
    record.admin
}

pub fn list_tokens(db: &Database) -> Vec<TokenRecord> {
    db.list_tokens()
}

pub fn revoke_token(db: &Database, prefix: &str) -> bool {
    db.revoke_token_by_prefix(prefix)
}

pub fn validate_bearer(db: &Database, auth_header: &str) -> Option<TokenRecord> {
    let token = auth_header.strip_prefix("Bearer ")?;
    let hash = hash_token(token);
    let record = db.validate_token(&hash)?;
    // Expired tokens are invalid: the caller sees a plain 401.
    if let Some(expires_at) = &record.expires_at {
        let deadline = chrono::DateTime::parse_from_rfc3339(expires_at).ok()?;
        if chrono::Utc::now() > deadline {
            return None;
        }
    }
    Some(record)
}

pub fn token_has_scope(record: &TokenRecord, package_name: &str) -> bool {
    match &record.scope {
        None => true,
        Some(scope) => {
            package_name == scope
                || package_name
                    .strip_prefix(scope)
                    .is_some_and(|rest| rest.starts_with('/'))
        }
    }
}

pub(crate) fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

fn generate_raw_token() -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    let bytes: [u8; 32] = rng.random();
    format!("sfr_{}", hex::encode(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token(scope: Option<&str>) -> TokenRecord {
        TokenRecord {
            token_hash: "hash".to_string(),
            scope: scope.map(str::to_string),
            label: "test".to_string(),
            created_at: "2026-01-01".to_string(),
            expires_at: None,
            admin: false,
        }
    }

    #[test]
    fn expired_token_fails_validation() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(&dir.path().join("t.db")).unwrap();
        // expires_in_days = Some(0) expires immediately
        let raw = create_token(&db, None, "short-lived", Some(0), false);
        let header = format!("Bearer {}", raw);
        assert!(
            validate_bearer(&db, &header).is_none(),
            "expired token must not validate"
        );
    }

    #[test]
    fn unexpired_token_validates() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(&dir.path().join("t.db")).unwrap();
        let raw = create_token(&db, None, "normal", Some(90), false);
        let header = format!("Bearer {}", raw);
        assert!(validate_bearer(&db, &header).is_some());
    }

    #[test]
    fn admin_flag_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(&dir.path().join("t.db")).unwrap();
        let raw = create_token(&db, None, "root", Some(90), true);
        let record = validate_bearer(&db, &format!("Bearer {}", raw)).unwrap();
        assert!(is_admin(&record));
    }

    #[test]
    fn token_without_scope_grants_all() {
        assert!(token_has_scope(&token(None), "web"));
        assert!(token_has_scope(&token(None), "webui"));
    }

    #[test]
    fn exact_scope_match_allowed() {
        assert!(token_has_scope(&token(Some("web")), "web"));
    }

    #[test]
    fn scope_prefix_collision_denied() {
        assert!(!token_has_scope(&token(Some("web")), "webui"));
    }

    #[test]
    fn package_under_scope_allowed() {
        assert!(token_has_scope(&token(Some("web")), "web/sub"));
        assert!(token_has_scope(&token(Some("web")), "web/sub/deep"));
    }

    #[test]
    fn package_outside_scope_denied() {
        assert!(!token_has_scope(&token(Some("web")), "other"));
        assert!(!token_has_scope(&token(Some("web")), "websub"));
    }

    #[test]
    fn npm_style_scope_matches_only_own_packages() {
        assert!(token_has_scope(&token(Some("@web")), "@web/ui"));
        assert!(!token_has_scope(&token(Some("@web")), "@webui/x"));
    }
}
