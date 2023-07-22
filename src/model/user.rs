use axum_login::secrecy::SecretVec;
use axum_login::AuthUser;
use serde::Serialize;

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

impl SimpleUser {
    pub fn from_user(user: &User) -> Self {
        Self {
            user_id: user.id.clone(),
            display_name: user.name.clone(),
        }
    }
}

impl AuthUser<String> for User {
    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn get_password_hash(&self) -> SecretVec<u8> {
        SecretVec::new(self.password_hash.clone().into())
    }
}

pub const PUBLIC_USER_ID: &str = "public";
