use axum_login::secrecy::SecretVec;
use axum_login::AuthUser;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub user_name: String,
    pub name: String,
    pub password_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimpleUser {
    pub display_name: String,
    pub user_name: String,
}

impl SimpleUser {
    pub fn from_user(user: &User) -> Self {
        SimpleUser {
            display_name: user.name.clone(),
            user_name: user.user_name.clone(),
        }
    }
}

impl AuthUser<String> for User {
    fn get_id(&self) -> String {
        self.user_name.clone()
    }

    fn get_password_hash(&self) -> SecretVec<u8> {
        SecretVec::new(self.password_hash.clone().into())
    }
}
