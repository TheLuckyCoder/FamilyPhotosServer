use crate::utils::file_storage::FileStorage;

pub mod db_utils;
pub mod file_storage;
pub mod data_init;

#[derive(Clone)]
pub struct AppState {
    pub db: actix::Addr<crate::db::DbActor>,
    pub storage: FileStorage,
}
