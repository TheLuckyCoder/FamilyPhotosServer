use crate::model::user::{User, UserCredentials};
use crate::utils::password_hash::validate_credentials;
use argon2::password_hash;
use async_trait::async_trait;
use axum_login::{AuthnBackend, UserId};
use sqlx::{query, query_as, Error, PgPool};
use tokio::task;

#[derive(Clone)]
pub struct UsersRepository {
    pool: PgPool,
}

impl UsersRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_user<T: AsRef<str>>(&self, user_name: T) -> Option<User> {
        query_as!(
            User,
            "select * from users where id = $1",
            user_name.as_ref()
        )
        .fetch_optional(&self.pool)
        .await
        .ok()?
    }

    pub async fn get_users(&self) -> Result<Vec<User>, Error> {
        query_as!(User, "select * from users")
            .fetch_all(&self.pool)
            .await
    }

    pub async fn insert_user(&self, user: &User) -> Result<(), Error> {
        query!(
            "insert into users (id, name, password_hash) values ($1, $2, $3)",
            user.id,
            user.name,
            user.password_hash
        )
        .execute(&self.pool)
        .await
        .map(|_| ())
    }

    pub async fn delete_user<T: AsRef<str>>(&self, user_name: T) -> Result<(), Error> {
        query!("delete from users where id = $1", user_name.as_ref())
            .execute(&self.pool)
            .await
            .map(|_| ())
    }
}

#[async_trait]
impl AuthnBackend for UsersRepository {
    type User = User;
    type Credentials = UserCredentials;
    type Error = password_hash::Error;

    async fn authenticate(
        &self,
        creds: Self::Credentials,
    ) -> Result<Option<Self::User>, Self::Error> {
        if let Some(user) = self.get_user(creds.user_id).await {
            return task::spawn_blocking(|| {
                Ok(
                    if validate_credentials(creds.password, &user.password_hash)? {
                        Some(user)
                    } else {
                        None
                    },
                )
            })
            .await
            .expect("Password verification failed unexpectedly");
        }

        Ok(None)
    }

    async fn get_user(&self, user_id: &UserId<Self>) -> Result<Option<Self::User>, Self::Error> {
        Ok(self.get_user(user_id).await)
    }
}
