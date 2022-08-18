use std::fs;
use walkdir::WalkDir;
use crate::{AppState, GetUsers, User};
use crate::model::photo::Photo;

pub struct DataInit;

impl DataInit {
    pub async fn start(app_state: &AppState) -> std::io::Result<()> {
        let db = &app_state.db;
        let storage = &app_state.storage;

        let users: Vec<User> = match db.send(GetUsers).await {
            Ok(Ok(users)) => users,
            _ => panic!("Could not load users")
        };
        println!("Users: {:?}", users);

        for user in &users {
            let user_path = storage.resolve(&user.user_name);
            if !user_path.exists() {
                fs::create_dir(user_path)?
            } else {
                let mut photos = vec![];
                let walkdir = WalkDir::new(user_path)
                    .max_depth(2)
                    .contents_first(true);

                for result in walkdir.into_iter() {
                    let entry = result?;
                    photos.push(Photo {
                        id: 0,
                        owner: user.id,
                        name: entry.file_name().to_string_lossy().to_string(),
                        time_created: Default::default(),
                        file_size: fs::metadata(entry.path()).map_or(0i64, |data| data.len() as i64),
                        folder: if entry.depth() == 2 { Some(entry.path().parent().unwrap().file_name().unwrap().to_string_lossy().to_string()) } else { None },
                    })
                };
            }
        }

        Ok(())
    }
}