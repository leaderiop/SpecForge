use crate::db::Database;
use crate::storage::LocalStorage;

pub struct AppState {
    pub database: Database,
    pub storage: LocalStorage,
}
