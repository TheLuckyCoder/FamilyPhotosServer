fn read_env_var(var_name: &str) -> String {
    std::env::var(var_name).unwrap_or_else(|_| panic!("{var_name} must be set!"))
}

pub struct EnvVariables {
    pub skip_scanning: bool,
    pub server_port: u16,
    pub use_https: bool,
    pub database_url: String,
    pub storage_path: String,
    pub ssl_private_key_path: String,
    pub ssl_certs_path: String,
}

impl EnvVariables {
    pub fn init() {
        dotenv::dotenv().ok();
        if std::env::var("RUST_LOG").is_err() {
            eprintln!("Logging is disabled, set RUST_LOG to enable logging")
        }
        std::env::set_var("RUST_BACKTRACE", "1");
    }
    
    pub fn get_all() -> Self {
        Self {
            skip_scanning: read_env_var("SKIP_SCANNING").parse().unwrap(),
            server_port: read_env_var("SERVER_PORT")
                .parse()
                .expect("SERVER_PORT must be a valid port number!"),
            use_https: read_env_var("USE_HTTPS").parse().unwrap(),
            database_url: read_env_var("DATABASE_URL"),
            storage_path: read_env_var("STORAGE_PATH"),
            ssl_private_key_path: read_env_var("SSL_PRIVATE_KEY_PATH"),
            ssl_certs_path: read_env_var("SSL_CERTS_PATH"),
        }
    }
}
