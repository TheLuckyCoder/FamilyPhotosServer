use chrono::naive::serde::ts_milliseconds;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};

use crate::model::user::User;
use crate::schema::photos;

#[derive(Debug, Clone, PartialEq, Eq, Identifiable, AsChangeset, Queryable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = photos, treat_none_as_null = true)]
pub struct Photo {
    pub id: i64,
    pub owner: i64,
    pub name: String,
    #[serde(rename = "timeCreated", with = "ts_milliseconds")]
    pub time_created: chrono::NaiveDateTime,
    #[serde(rename = "fileSize")]
    pub file_size: i64,
    pub folder: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PhotoBody {
    pub owner: i64,
    pub name: String,
    pub time_created: chrono::NaiveDateTime,
    pub file_size: i64,
    pub folder: Option<String>,
}

impl Photo {
    pub fn partial_photo_path(&self, user: &User) -> Result<String, String> {
        if self.owner != user.id {
            return Err("Photo owner does not match user id".to_string());
        }

        let folder = match self.folder.clone() {
            None => String::new(),
            Some(folder) => folder + "/"
        };

        Ok(format!("photos/{}/{}{}", user.user_name, folder, self.name))
    }
}