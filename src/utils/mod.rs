use crate::utils::file_storage::FileStorage;

pub mod data_scan;
pub mod env_reader;
pub mod file_storage;
pub mod password_hash;
pub mod status_error;
pub mod thumbnail;

#[derive(Clone)]
pub struct AppState {
    pub db: actix::Addr<crate::db::DbActor>,
    pub storage: FileStorage,
}
