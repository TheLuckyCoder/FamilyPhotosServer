use clap::{Parser, Subcommand};

use crate::db::users_db::{DeleteUser, GetUsers, InsertUser};
use crate::model::user::SimpleUser;
use crate::utils::password_hash::generate_password;
use crate::utils::AppState;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    commands: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(subcommand)]
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
        display_name: String,
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
pub async fn run_cli(app_state: &AppState) -> bool {
    let cli = Cli::parse();
    let db = app_state.db.clone();

    let cmd = cli.commands;
    if cmd.is_none() {
        return false;
    }

    match cmd.unwrap() {
        Commands::Users(user_command) => match user_command {
            UsersCommand::Create {
                user_name,
                display_name,
                password,
            } => {
                let user_result = db
                    .send(InsertUser::WithoutId {
                        user_name,
                        display_name,
                        hashed_password: password.unwrap_or_else(generate_password),
                    })
                    .await;

                match user_result {
                    Ok(Ok(user)) => println!("User created: {:?}", SimpleUser::from_user(&user)),
                    _ => eprintln!("Error creating user"),
                }
            }
            UsersCommand::List => match db.send(GetUsers).await {
                Ok(Ok(users)) => {
                    println!("Users:");
                    for user in users {
                        println!("\t{:?}", SimpleUser::from_user(&user));
                    }
                }
                _ => eprintln!("Error listing users"),
            },
            UsersCommand::Remove { user_name } => {
                match db
                    .send(DeleteUser {
                        user_name: user_name.clone(),
                    })
                    .await
                {
                    Ok(Ok(_)) => println!("Deleted user with user name: {user_name}"),
                    _ => eprintln!("Failed to remove user with user name: {user_name}"),
                }
            }
        },
    }

    true
}
