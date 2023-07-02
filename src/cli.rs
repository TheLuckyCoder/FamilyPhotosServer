use clap::{Parser, Subcommand};

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
    /// Add or remove users
    Users(UsersCommand),
}

#[derive(Subcommand)]
enum UsersCommand {
    /// Create a new user
    Create {
        #[arg(short, long)]
        /// The name used for login and in the filesystem
        user_name: String,
        #[arg(short, long)]
        /// The name visible to the user
        name: String,
        #[arg(short, long)]
        /// Random password will be generated if not provided
        password: Option<String>,
    },
    /// List all users
    List,
    /// Remove an existing user
    Remove {
        #[arg(short, long)]
        user_name: String,
    },
}

/**
 * @return true if the program should exit
 */
pub async fn run_cli(state: &AppState) -> bool {
    let cli = Cli::parse();

    let cmd = cli.commands;
    if cmd.is_none() {
        return false;
    }

    match cmd.unwrap() {
        Commands::Users(user_command) => match user_command {
            UsersCommand::Create {
                user_name,
                name,
                password,
            } => {
                let final_password = &password.unwrap_or_else(generate_random_password);
                let user_result = state
                    .users_repo
                    .insert_user(User {
                        user_name,
                        name,
                        password_hash: generate_hash_from_password(final_password),
                    })
                    .await;

                match user_result {
                    Ok(user) => println!(
                        "User created with {{user name=\"{}\", name=\"{}\", password=\"{}\"}}",
                        user.user_name, user.name, final_password
                    ),
                    _ => eprintln!("Error creating user"),
                }
            }
            UsersCommand::List => match state.users_repo.get_users().await {
                Ok(users) => {
                    println!("Users:");
                    for user in users {
                        println!(
                            "\t{{user name=\"{}\", name=\"{}\"}}",
                            user.user_name, user.name
                        );
                    }
                }
                _ => eprintln!("Error listing users"),
            },
            UsersCommand::Remove { user_name } => {
                match state.users_repo.delete_user(&user_name).await {
                    Ok(_) => println!("Deleted user with user name: {user_name}"),
                    _ => eprintln!("Failed to remove user with user name: {user_name}"),
                }
            }
        },
    }

    true
}
