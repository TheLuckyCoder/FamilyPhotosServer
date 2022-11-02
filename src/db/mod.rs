use actix::{Actor, SyncContext};
use diesel::{
    r2d2::{ConnectionManager, Pool},
    PgConnection,
};
use rand_hc::Hc128Rng;
use std::sync::Mutex;

pub struct DbActor(
    pub Pool<ConnectionManager<PgConnection>>,
    pub Mutex<Hc128Rng>,
);

impl Actor for DbActor {
    type Context = SyncContext<Self>;
}

pub fn get_pool(db_url: &str) -> Pool<ConnectionManager<PgConnection>> {
    let manager = ConnectionManager::<PgConnection>::new(db_url);
    Pool::builder()
        .build(manager)
        .expect("Error building a connection pool")
}

pub mod photos;
pub mod users;
