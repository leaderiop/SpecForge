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
        Some(scope) => package_name.starts_with(scope),
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
