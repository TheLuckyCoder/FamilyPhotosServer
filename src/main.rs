use actix::SyncArbiter;
use actix_web::{App, HttpServer, middleware};
use actix_web::middleware::{Logger, TrailingSlash};
use actix_web::web::Data;
use actix_web_httpauth::middleware::HttpAuthentication;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    std::env::set_var("RUST_LOG", "debug");
    std::env::set_var("RUST_BACKTRACE", "1");
    env_logger::init();

    let manager = SyncArbiter::start(2, move || {
        let pool = get_pool(database_url.as_str());
        DbActor(pool)
    });

    HttpServer::new(move || {
        let logger = Logger::default();
        let auth = HttpAuthentication::basic(auth_validator);

        App::new()
            .wrap(logger)
            .wrap(middleware::NormalizePath::new(TrailingSlash::Trim))
            .app_data(Data::new(AppState {
                db: manager.clone()
            }))
    })
        .bind(("127.0.0.1", 5000))?
        .run()
        .await
}
