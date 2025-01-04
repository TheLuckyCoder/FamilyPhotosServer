use std::path::PathBuf;

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
    pub server_port: u16,
    pub storage_path: PathBuf,
    pub database_url: String,
    pub previews_path: PathBuf,
    pub scan_new_files: bool,
}

impl EnvVariables {
    pub fn get_all() -> Self {
        dotenvy::dotenv().ok();
        
        let storage_path = PathBuf::from(required_env_var("STORAGE_PATH"));
        if storage_path.exists() && !storage_path.is_dir() {
            panic!("STORAGE_PATH must be a directory!")
        }

        let previews_path = std::env::var("PREVIEWS_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| storage_path.join(".previews"));
        
        if previews_path.exists() && !previews_path.is_dir() {
            panic!("PREVIEWS_PATH must be a directory!")
        }

        let database_url = std::env::var("DATABASE_URL")
            .map(PathBuf::from)
            .unwrap_or_else(|_| storage_path.join(".familyphotos.db"));
        
        if database_url.exists() && !database_url.is_file() {
            panic!("DATABASE_URL must be a file!")
        }

        Self {
            server_port: required_env_var("SERVER_PORT")
                .parse()
                .expect("SERVER_PORT must be a valid port number!"),
            storage_path,
            database_url: database_url.to_string_lossy().to_string(),
            previews_path,
            scan_new_files: optional_env_var("SCAN_NEW_FILES", true),
        }
    }
}
