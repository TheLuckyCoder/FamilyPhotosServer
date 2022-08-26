use crate::schema::users;
use diesel::{Insertable, Queryable};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Queryable, Insertable, Deserialize)]
#[diesel(table_name = users)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: i64,
    pub display_name: String,
    pub user_name: String,
    pub password: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimpleUser {
    pub id: i64,
    pub display_name: String,
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