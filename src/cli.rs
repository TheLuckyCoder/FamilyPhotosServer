use crate::{file_scan, thumbnail};
use clap::{Parser, Subcommand};
use sqlx::PgPool;

use crate::http::AppState;
use crate::model::user::User;
use crate::utils::password_hash::{generate_hash_from_password, generate_random_password};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    commands: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(subcommand)]
    /// Manage users
    Users(UsersCommand),
    #[command(subcommand)]
    /// Manage Photos
    Photos(PhotosCommand),
    #[command(subcommand)]
    /// Manage Sessions
    Sessions(SessionsCommand),
}

#[derive(Subcommand)]
enum UsersCommand {
    /// Create a new user
    Create {
        #[arg(short, long)]
        /// The name used for login and the filesystem folder name
        user_id: String,
        #[arg(short, long)]
        /// The name visible to the user
        name: String,
        #[arg(short, long)]
        /// Random password will be generated if not provided
        password: Option<String>,
    },
    /// List all users and their respective photo count
    List,
    /// Remove an existing user
    Remove {
        #[arg(short, long)]
        user_name: String,
    },
}

#[derive(Subcommand)]
enum PhotosCommand {
    /// Trigger a manual scan of the filesystem
    ScanPhotos,
    /// Trigger a manual generation of thumbnails
    GenerateThumbnails,
}

#[derive(Subcommand)]
enum SessionsCommand {
    /// Clear all sessions
    Clear,
}

/**
 * @return true if the program should exit
 */
pub async fn run_cli(pool: &PgPool, state: &AppState) -> bool {
    let cli = Cli::parse();

    let cmd = cli.commands;
    if cmd.is_none() {
        return false;
    }

    match cmd.unwrap() {
        Commands::Users(command) => user_commands(state, command).await,
        Commands::Photos(command) => photos_commands(state, command).await,
        Commands::Sessions(command) => sessions_commands(pool, command).await,
    };

    true
}

async fn user_commands(state: &AppState, command: UsersCommand) {
    match command {
        UsersCommand::Create {
            user_id,
            name,
            password,
        } => {
            let final_password = &password.unwrap_or_else(generate_random_password);
            let user = User {
                id: user_id,
                name,
                password_hash: generate_hash_from_password(final_password),
            };

            let user_result = state.users_repo.insert_user(&user).await;

            match user_result {
                Ok(_) => println!(
                    "User created with user name=\"{}\", name=\"{}\", password=\"{}\"",
                    user.id, user.name, final_password
                ),
                _ => eprintln!("Error creating user"),
            }
        }
        UsersCommand::List => {
            println!(
                "| {0: <10} | {1: <10} | {2: <10} |",
                "User Id", "Name", "Photos Count"
            );

            let users = state
                .users_repo
                .get_users()
                .await
                .expect("Failed to get users");

            for user in users {
                let count = state
                    .photos_repo
                    .get_photos_by_user(user.id.as_str())
                    .await
                    .expect("Failed to get photos count")
                    .len();

                println!(
                    "| {0: <10} | {1: <10} | {2: <10} |",
                    user.id, user.name, count
                );
            }
        }
        UsersCommand::Remove { user_name } => {
            match state.users_repo.delete_user(&user_name).await {
                Ok(_) => println!("Deleted user with user name: {user_name}"),
                _ => eprintln!("Failed to remove user with user name: {user_name}"),
            }
        }
    }
}

async fn photos_commands(state: &AppState, command: PhotosCommand) {
    match command {
        PhotosCommand::ScanPhotos => {
            file_scan::scan_new_files(state.clone())
                .await
                .expect("Failed to join task");
        }
        PhotosCommand::GenerateThumbnails => {
            match thumbnail::generate_all_foreground(state).await {
                Ok(_) => println!("Thumbnail generation finished"),
                Err(e) => eprintln!("Thumbnail generation failed: {e}"),
            }
        }
    }
}

async fn sessions_commands(pool: &PgPool, command: SessionsCommand) {
    match command {
        SessionsCommand::Clear => sqlx::query!("delete from session")
            .execute(pool)
            .await
            .expect("Failed to clear sessions"),
    };
}
