use axum_login::AuthUser;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct User {
    pub id: String,
    pub name: String,
    pub password_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SimpleUser {
    pub user_id: String,
    pub display_name: String,
}

impl From<User> for SimpleUser {
    fn from(value: User) -> Self {
        Self {
            user_id: value.id,
            display_name: value.name,
        }
    }
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserCredentials {
    pub user_id: String,
    pub password: String,
}

impl AuthUser for User {
    type Id = String;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }

    fn session_auth_hash(&self) -> &[u8] {
        self.password_hash.as_bytes()
    }
}

pub const PUBLIC_USER_ID: &str = "public";
