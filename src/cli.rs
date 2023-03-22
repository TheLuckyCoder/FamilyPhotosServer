use clap::{Parser, Subcommand};

use crate::db::users::{DeleteUser, GetUsers, InsertUser};
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
    /// Create a new user with a random password
    Add {
        #[arg(short, long)]
        user_name: String,
        #[arg(short, long)]
        display_name: String,
    },
    /// List all users
    List,
    /// Remove an existing user
    Remove {
        #[arg(short, long)]
        user_name: String,
    },
}

pub async fn run_cli(app_state: &AppState) {
    let cli = Cli::parse();
    let db = app_state.db.clone();

    let cmd = cli.commands;
    if cmd.is_none() {
        return;
    }

    match cmd.unwrap() {
        Commands::Users(user_command) => match user_command {
            UsersCommand::Add {
                user_name,
                display_name,
            } => {
                let user_result = db
                    .send(InsertUser::WithoutId {
                        user_name,
                        display_name,
                        hashed_password: generate_password(),
                    })
                    .await;

                match user_result {
                    Ok(Ok(user)) => println!("User created: {:?}", user),
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
                match db.send(DeleteUser{ user_name: user_name.clone() }).await {
                    Ok(Ok(_)) => println!("Deleted user with user name: {user_name}"),
                    _ => eprintln!("Failed to remove user with user name: {user_name}"),
                }
            }
        },
    }
}
