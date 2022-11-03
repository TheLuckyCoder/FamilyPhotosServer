fn required_env_var(var_name: &str) -> String {
    std::env::var(var_name).unwrap_or_else(|_| panic!("{var_name} must be set!"))
}

fn optional_env_var<V>(var_name: &str, default_value: V) -> V
where
    V: std::str::FromStr + Copy,
{
    std::env::var(var_name).map_or(default_value, |v| v.parse::<V>().unwrap_or(default_value))
}

pub struct EnvVariables {
    pub skip_scanning: bool,
    pub server_port: u16,
    pub use_https: bool,
    pub database_url: String,
    pub storage_path: String,
    pub ssl_private_key_path: Option<String>,
    pub ssl_certs_path: Option<String>,
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
            skip_scanning: optional_env_var("SKIP_SCANNING", false),
            server_port: required_env_var("SERVER_PORT")
                .parse()
                .expect("SERVER_PORT must be a valid port number!"),
            use_https: optional_env_var("SKIP_SCANNING", false),
            database_url: required_env_var("DATABASE_URL"),
            storage_path: required_env_var("STORAGE_PATH"),
            ssl_private_key_path: std::env::var("SSL_PRIVATE_KEY_PATH").ok(),
            ssl_certs_path: std::env::var("SSL_CERTS_PATH").ok(),
        }
    }
}
