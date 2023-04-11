use actix::{Actor, SyncContext};
use diesel::pg::Pg;
use diesel::{
    r2d2::{ConnectionManager, Pool},
    PgConnection,
};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use rand::SeedableRng;
use rand_hc::Hc128Rng;
use std::error::Error;
use std::sync::Mutex;

pub struct DbActor(
    pub Pool<ConnectionManager<PgConnection>>,
    pub Mutex<Hc128Rng>,
);

impl Actor for DbActor {
    type Context = SyncContext<Self>;
}

impl DbActor {
    pub fn new(db_url: &str) -> Self {
        let pool = get_pool(db_url);
        let mut conn = pool.get().expect("Unable to get a database connection");
        run_migrations(&mut conn).expect("Unable to run database migrations");

        Self(pool, Mutex::new(Hc128Rng::from_entropy()))
    }
}

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

fn get_pool<S: Into<String>>(db_url: S) -> Pool<ConnectionManager<PgConnection>> {
    let manager = ConnectionManager::<PgConnection>::new(db_url);
    Pool::builder()
        .build(manager)
        .expect("Error building a connection pool")
}

fn run_migrations(
    connection: &mut impl MigrationHarness<Pg>,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    connection.run_pending_migrations(MIGRATIONS)?;

    Ok(())
}

pub mod photos_db;
pub mod users_db;
