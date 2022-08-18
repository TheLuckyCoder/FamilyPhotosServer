use std::sync::Mutex;
use actix::{Actor, SyncContext};
use diesel::{
    PgConnection,
    r2d2::{ConnectionManager, Pool},
};
use rand_hc::Hc128Rng;

pub struct DbActor(pub Pool<ConnectionManager<PgConnection>>, pub Mutex<Hc128Rng>);

impl Actor for DbActor {
    type Context = SyncContext<Self>;
}

pub mod photos;
pub mod users;