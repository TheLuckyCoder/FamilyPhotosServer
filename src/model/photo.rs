use chrono::naive::serde::ts_milliseconds;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};

use crate::model::user::User;
use crate::schema::photos;

#[derive(Debug, Clone, PartialEq, Eq, Identifiable, AsChangeset, Queryable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = photos, treat_none_as_null = true)]
#[serde(rename_all = "camelCase")]
pub struct Photo {
    pub id: i64,
    pub owner: i64,
    pub name: String,
    #[serde(with = "ts_milliseconds")]
    pub time_created: chrono::NaiveDateTime,
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
    pub fn full_name(&self) -> String {
        let folder_path = match self.folder.clone() {
            None => String::new(),
            Some(folder) => folder + "/"
        };

        folder_path + self.name.as_str()
    }

    pub fn partial_path(&self, user: &User) -> Result<String, String> {
        if self.owner != user.id {
            return Err("Photo owner does not match user id".to_string());
        }

        Ok(format!("{}/{}", user.user_name, self.full_name()))
    }
}