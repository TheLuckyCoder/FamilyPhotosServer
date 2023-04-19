extern crate diesel;

use std::net::SocketAddr;
use std::sync::Arc;

use crate::db::*;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use env_logger::Env;
use rand::SeedableRng;
use rand_hc::Hc128Rng;
use rustls::{Certificate, PrivateKey, ServerConfig};
use rustls_pemfile::{certs, pkcs8_private_keys};
#[cfg(not(target_env = "msvc"))]
use tikv_jemallocator::Jemalloc;
use tokio::sync::Mutex;

use crate::db::users_db::{GetUsers, InsertUser};
use crate::http::AppState;
use crate::model::user::User;
use crate::utils::env_reader::EnvVariables;
use crate::utils::file_storage::FileStorage;
use crate::utils::password_hash::{generate_password, get_hash_from_password};

mod cli;
mod db;
mod file_scan;
mod http;
mod model;
mod schema;
mod thumbnail;
mod utils;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

/*static mut USERS: Vec<User> = Vec::new();

async fn any_user_auth_validator(
    req: ServiceRequest,
    credentials: basic::BasicAuth,
) -> Result<ServiceRequest, (Error, ServiceRequest)> {
    let found = unsafe { USERS.clone() }
        .into_iter()
        .find(|user| user.user_name == credentials.user_id());

    if let Some(user) = found {
        if let Some(password) = credentials.password() {
            if get_hash_from_password(&password.to_string()) == user.password {
                return Ok(req);
            }
        }
    }

    Err((Error::from(AuthenticationError::new(Basic::new())), req))
}*/

#[tokio::main]
async fn main() {
    EnvVariables::init();
    let vars = EnvVariables::get_all();
    env_logger::Builder::from_env(Env::default())
        .format_timestamp(None)
        .init();

    let config =
        AsyncDieselConnectionManager::<diesel_async::AsyncPgConnection>::new(vars.database_url);
    let pool = Pool(
        bb8::Pool::builder()
            .build(config)
            .await
            .expect("Error building a connection pool"),
        Arc::new(Mutex::new(Hc128Rng::from_entropy())),
    );

    let app_state = AppState {
        pool,
        storage: FileStorage::new(vars.storage_path, vars.thumbnail_path),
    };

    if cli::run_cli(&app_state.pool).await {
        return;
    }

    // Scan the storage directory for new photos in the background
    if vars.scan_new_files {
        file_scan::scan_new_files(app_state.clone()).await;
    }

    if vars.generate_thumbnails_background {
        match thumbnail::generate_background(&app_state.clone()).await {
            Ok(_) => log::info!("Background thumbnail generation finished"),
            Err(e) => log::error!("Could not start background thumbnail generation: {e}"),
        }
    }

    {
        let mut users: Vec<User> = match app_state.pool.send(GetUsers).await {
            Ok(users) => users,
            _ => panic!("Could not load users"),
        };

        if users.is_empty() {
            let public_user = User {
                id: 1,
                display_name: "Public".to_string(),
                user_name: "public".to_string(),
                password: generate_password(),
            };

            println!(
                "No users found, creating public user with password: {}",
                public_user.password
            );

            match app_state.pool.send(InsertUser::WithId(public_user)).await {
                Ok(user) => users = vec![user],
                _ => panic!("Failed to create public user"),
            };
        }

        /*unsafe {
            USERS = users;
        }*/
    }

    log::info!("Server listening on port {}", vars.server_port);

    let addr = SocketAddr::from(([127, 0, 0, 1], vars.server_port));
    axum::Server::bind(&addr)
        .serve(http::router(app_state).into_make_service())
        .await
        .expect("Failed to start axum server");

    /*if vars.use_https {
        let config = load_rustls_config(
            vars.ssl_certs_path
                .expect("SSL_CERTS_PATH variable is missing"),
            vars.ssl_private_key_path
                .expect("SSL_PRIVATE_KEY_PATH variable is missing"),
        )?;

        log::info!("Server configured successfully in HTTPS mode");
        server
            .bind_rustls(("127.0.0.1", vars.server_port), config)?
            .run()
            .await
    } else {
        log::info!("Server configured successfully in HTTP mode");
        server.bind(("127.0.0.1", vars.server_port))?.run().await
    }*/
}

/*fn load_rustls_config(certs_path: String, key_path: String) -> std::io::Result<ServerConfig> {
    // init server config builder with safe defaults
    let config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth();

    // load TLS key/cert files
    let cert_file = &mut BufReader::new(File::open(certs_path)?);
    let key_file = &mut BufReader::new(File::open(key_path)?);

    // convert files to key/cert objects
    let cert_chain = certs(cert_file)
        .unwrap()
        .into_iter()
        .map(Certificate)
        .collect();
    let mut keys: Vec<PrivateKey> = pkcs8_private_keys(key_file)
        .unwrap()
        .into_iter()
        .map(PrivateKey)
        .collect();

    // exit if no keys could be parsed
    if keys.is_empty() {
        eprintln!("Could not locate PKCS 8 private keys.");
        std::process::exit(1);
    }

    config
        .with_single_cert(cert_chain, keys.remove(0))
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Could not load TLS config: {}", e),
            )
        })
}*/
