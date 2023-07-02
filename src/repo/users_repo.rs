use crate::model::user::User;
use sqlx::{query, query_as, Error, PgPool};

#[derive(Clone)]
pub struct UsersRepository {
    pool: PgPool,
}

impl UsersRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_user<T: AsRef<str>>(&self, user_name: T) -> User {
        query_as!(
            User,
            "select * from users where user_name = $1",
            user_name.as_ref()
        )
        .fetch_one(&self.pool)
        .await
        .unwrap()
    }

    pub async fn get_users(&self) -> Result<Vec<User>, Error> {
        query_as!(User, "select * from users")
            .fetch_all(&self.pool)
            .await
    }

    pub async fn insert_user(&self, user: User) -> Result<User, Error> {
        query_as!(
            User,
            "insert into users (user_name, name, password_hash) values ($1, $2, $3) returning *",
            user.user_name,
            user.name,
            user.password_hash
        )
        .fetch_one(&self.pool)
        .await
    }

    pub async fn delete_user<T: AsRef<str>>(&self, user_name: T) -> Result<(), Error> {
        query!("delete from users where user_name = $1", user_name.as_ref())
            .execute(&self.pool)
            .await
            .map(|_| ())
    }
}
