use crate::db::utils::{Handler, Pool};
use crate::model::user::User;
use crate::schema::users::dsl;
use crate::schema::users::dsl::users;
use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

pub enum InsertUser {
    WithId(User),
    WithoutId {
        user_name: String,
        display_name: String,
        hashed_password: String,
    },
}

pub struct GetUsers;

pub enum GetUser {
    Id(i64),
    UserName(String),
}

pub struct DeleteUser {
    pub(crate) user_name: String,
}

#[async_trait]
impl Handler<InsertUser> for Pool {
    type Result = QueryResult<User>;

    async fn send(&self, msg: InsertUser) -> Self::Result {
        let mut conn = self.0.get().await.expect("Unable to get a connection");

        match msg {
            InsertUser::WithId(user) => diesel::insert_into(users)
                .values(user)
                .get_result::<User>(&mut conn),
            InsertUser::WithoutId {
                user_name,
                display_name,
                hashed_password,
            } => diesel::insert_into(users)
                .values((
                    dsl::user_name.eq(user_name),
                    dsl::display_name.eq(display_name),
                    dsl::password.eq(hashed_password),
                ))
                .get_result::<User>(&mut conn),
        }
        .await
    }
}

#[async_trait]
impl Handler<GetUsers> for Pool {
    type Result = QueryResult<Vec<User>>;

    async fn send(&self, _: GetUsers) -> Self::Result {
        let mut conn = self.0.get().await.expect("Unable to get a connection");
        users.get_results::<User>(&mut conn).await
    }
}

#[async_trait]
impl Handler<GetUser> for Pool {
    type Result = QueryResult<User>;

    async fn send(&self, msg: GetUser) -> Self::Result {
        let mut conn = self.0.get().await.expect("Unable to get a connection");

        match msg {
            GetUser::Id(user_id) => users
                .filter(dsl::id.eq(user_id))
                .get_result::<User>(&mut conn),
            GetUser::UserName(name) => users
                .filter(dsl::user_name.eq(name))
                .get_result::<User>(&mut conn),
        }
        .await
    }
}

#[async_trait]
impl Handler<DeleteUser> for Pool {
    type Result = QueryResult<usize>;

    async fn send(&self, msg: DeleteUser) -> Self::Result {
        let mut conn = self.0.get().await.expect("Unable to get a connection");

        diesel::delete(users.filter(dsl::user_name.eq(msg.user_name)))
            .execute(&mut conn)
            .await
    }
}
