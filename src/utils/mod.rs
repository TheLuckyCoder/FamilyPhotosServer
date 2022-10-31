use crate::utils::file_storage::FileStorage;

pub mod data_scan;
pub mod db;
pub mod file_storage;
pub mod password_hash;
pub mod thumbnail;
pub mod env_reader;

#[derive(Clone)]
pub struct AppState {
    pub db: actix::Addr<crate::db::DbActor>,
    pub storage: FileStorage,
}
