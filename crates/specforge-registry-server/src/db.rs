use rusqlite::{Connection, params};
use std::path::Path;
use std::sync::Mutex;

pub struct Database {
    conn: Mutex<Connection>,
}

#[derive(Debug, Clone)]
pub struct PackageVersion {
    pub name: String,
    pub version: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub description: String,
    pub keywords: String,
    pub publisher: String,
    pub published_at: String,
}

#[derive(Debug, Clone)]
pub struct TokenRecord {
    pub token_hash: String,
    pub scope: Option<String>,
    pub label: String,
    pub created_at: String,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self, String> {
        let conn = Connection::open(path)
            .map_err(|e| format!("failed to open database: {}", e))?;

        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .map_err(|e| format!("failed to set pragmas: {}", e))?;

        let db = Self { conn: Mutex::new(conn) };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS packages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                version TEXT NOT NULL,
                sha256 TEXT NOT NULL,
                size_bytes INTEGER NOT NULL DEFAULT 0,
                description TEXT NOT NULL DEFAULT '',
                keywords TEXT NOT NULL DEFAULT '',
                publisher TEXT NOT NULL DEFAULT '',
                published_at TEXT NOT NULL DEFAULT (datetime('now')),
                yanked INTEGER NOT NULL DEFAULT 0,
                UNIQUE(name, version)
            );

            CREATE INDEX IF NOT EXISTS idx_packages_name ON packages(name);
            CREATE INDEX IF NOT EXISTS idx_packages_keywords ON packages(keywords);

            CREATE TABLE IF NOT EXISTS tokens (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                token_hash TEXT NOT NULL UNIQUE,
                scope TEXT,
                label TEXT NOT NULL DEFAULT 'default',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                revoked INTEGER NOT NULL DEFAULT 0
            );

            CREATE INDEX IF NOT EXISTS idx_tokens_hash ON tokens(token_hash);",
        )
        .map_err(|e| format!("migration failed: {}", e))?;
        Ok(())
    }

    pub fn insert_package(&self, pkg: &PackageVersion) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO packages (name, version, sha256, size_bytes, description, keywords, publisher, published_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                pkg.name,
                pkg.version,
                pkg.sha256,
                pkg.size_bytes,
                pkg.description,
                pkg.keywords,
                pkg.publisher,
                pkg.published_at,
            ],
        ).map_err(|e| {
            if e.to_string().contains("UNIQUE constraint") {
                format!("version {} already exists for {}", pkg.version, pkg.name)
            } else {
                format!("failed to insert package: {}", e)
            }
        })?;
        Ok(())
    }

    pub fn get_package_version(&self, name: &str, version: &str) -> Option<PackageVersion> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT name, version, sha256, size_bytes, description, keywords, publisher, published_at
             FROM packages WHERE name = ?1 AND version = ?2 AND yanked = 0",
            params![name, version],
            |row| {
                Ok(PackageVersion {
                    name: row.get(0)?,
                    version: row.get(1)?,
                    sha256: row.get(2)?,
                    size_bytes: row.get(3)?,
                    description: row.get(4)?,
                    keywords: row.get(5)?,
                    publisher: row.get(6)?,
                    published_at: row.get(7)?,
                })
            },
        ).ok()
    }

    pub fn get_package_versions(&self, name: &str) -> Vec<String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT version FROM packages WHERE name = ?1 AND yanked = 0 ORDER BY published_at")
            .unwrap();
        stmt.query_map(params![name], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect()
    }

    pub fn search(&self, query: &str, limit: u32) -> Vec<PackageVersion> {
        let conn = self.conn.lock().unwrap();
        let pattern = format!("%{}%", query);
        let mut stmt = conn
            .prepare(
                "SELECT name, version, sha256, size_bytes, description, keywords, publisher, published_at
                 FROM packages
                 WHERE yanked = 0 AND (name LIKE ?1 OR description LIKE ?1 OR keywords LIKE ?1)
                 GROUP BY name
                 HAVING version = MAX(version)
                 ORDER BY name
                 LIMIT ?2",
            )
            .unwrap();
        stmt.query_map(params![pattern, limit], |row| {
            Ok(PackageVersion {
                name: row.get(0)?,
                version: row.get(1)?,
                sha256: row.get(2)?,
                size_bytes: row.get(3)?,
                description: row.get(4)?,
                keywords: row.get(5)?,
                publisher: row.get(6)?,
                published_at: row.get(7)?,
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect()
    }

    pub fn yank_version(&self, name: &str, version: &str) -> bool {
        let conn = self.conn.lock().unwrap();
        let rows = conn
            .execute(
                "UPDATE packages SET yanked = 1 WHERE name = ?1 AND version = ?2",
                params![name, version],
            )
            .unwrap_or(0);
        rows > 0
    }

    // --- Token management ---

    pub fn insert_token(&self, token_hash: &str, scope: Option<&str>, label: &str) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO tokens (token_hash, scope, label) VALUES (?1, ?2, ?3)",
            params![token_hash, scope, label],
        )
        .map_err(|e| format!("failed to insert token: {}", e))?;
        Ok(())
    }

    pub fn validate_token(&self, token_hash: &str) -> Option<TokenRecord> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT token_hash, scope, label, created_at FROM tokens WHERE token_hash = ?1 AND revoked = 0",
            params![token_hash],
            |row| {
                Ok(TokenRecord {
                    token_hash: row.get(0)?,
                    scope: row.get(1)?,
                    label: row.get(2)?,
                    created_at: row.get(3)?,
                })
            },
        ).ok()
    }

    pub fn list_tokens(&self) -> Vec<TokenRecord> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT token_hash, scope, label, created_at FROM tokens WHERE revoked = 0 ORDER BY created_at")
            .unwrap();
        stmt.query_map([], |row| {
            Ok(TokenRecord {
                token_hash: row.get(0)?,
                scope: row.get(1)?,
                label: row.get(2)?,
                created_at: row.get(3)?,
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect()
    }

    pub fn revoke_token_by_prefix(&self, prefix: &str) -> bool {
        let conn = self.conn.lock().unwrap();
        let pattern = format!("{}%", prefix);
        let rows = conn
            .execute(
                "UPDATE tokens SET revoked = 1 WHERE token_hash LIKE ?1 AND revoked = 0",
                params![pattern],
            )
            .unwrap_or(0);
        rows > 0
    }
}
