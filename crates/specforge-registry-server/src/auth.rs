use sha2::{Digest, Sha256};

use crate::db::{Database, TokenRecord};

pub fn create_token(db: &Database, scope: Option<&str>, label: &str) -> String {
    let raw_token = generate_raw_token();
    let hash = hash_token(&raw_token);
    db.insert_token(&hash, scope, label)
        .expect("failed to store token");
    raw_token
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
    db.validate_token(&hash)
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

fn hash_token(token: &str) -> String {
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
        }
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
