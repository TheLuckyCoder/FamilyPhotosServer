use diesel::{Insertable, Queryable};
use serde::{Deserialize, Serialize};
use crate::schema::users;

#[derive(Debug, Clone, PartialEq, Eq, Queryable, Insertable, Deserialize)]
#[diesel(table_name = users)]
pub struct User {
    pub id: i64,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "userName")]
    pub user_name: String,
    pub password: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimpleUser {
    pub id: i64,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "userName")]
    pub user_name: String,
}

impl SimpleUser {
    pub fn from_user(user: &User) -> Self {
        SimpleUser {
            id: user.id,
            display_name: user.display_name.clone(),
            user_name: user.user_name.clone(),
        }
    }
}