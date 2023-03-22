use actix::{Handler, Message};
use diesel::prelude::*;

use crate::model::user::User;
use crate::schema::users::dsl;
use crate::schema::users::dsl::users;
use crate::DbActor;

#[derive(Message)]
#[rtype(result = "QueryResult<User>")]
pub enum InsertUser {
    WithId(User),
    WithoutId {
        user_name: String,
        display_name: String,
        hashed_password: String,
    },
}

#[derive(Message)]
#[rtype(result = "QueryResult<Vec<User>>")]
pub struct GetUsers;

#[derive(Message)]
#[rtype(result = "QueryResult<User>")]
pub enum GetUser {
    Id(i64),
    UserName(String),
}

#[derive(Message)]
#[rtype(result = "QueryResult<usize>")]
pub struct DeleteUser {
    pub(crate) user_name: String,
}

impl Handler<InsertUser> for DbActor {
    type Result = QueryResult<User>;

    fn handle(&mut self, msg: InsertUser, _: &mut Self::Context) -> Self::Result {
        let mut conn = self.0.get().expect("Unable to get a connection");

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
    }
}

impl Handler<GetUsers> for DbActor {
    type Result = QueryResult<Vec<User>>;

    fn handle(&mut self, _: GetUsers, _: &mut Self::Context) -> Self::Result {
        let mut conn = self.0.get().expect("Unable to get a connection");
        users.get_results::<User>(&mut conn)
    }
}

impl Handler<GetUser> for DbActor {
    type Result = QueryResult<User>;

    fn handle(&mut self, msg: GetUser, _: &mut Self::Context) -> Self::Result {
        let mut conn = self.0.get().expect("Unable to get a connection");

        match msg {
            GetUser::Id(user_id) => users
                .filter(dsl::id.eq(user_id))
                .get_result::<User>(&mut conn),
            GetUser::UserName(name) => users
                .filter(dsl::user_name.eq(name))
                .get_result::<User>(&mut conn),
        }
    }
}

impl Handler<DeleteUser> for DbActor {
    type Result = QueryResult<usize>;

    fn handle(&mut self, msg: DeleteUser, _: &mut Self::Context) -> Self::Result {
        let mut conn = self.0.get().expect("Unable to get a connection");

        diesel::delete(users.filter(dsl::user_name.eq(msg.user_name))).execute(&mut conn)
    }
}
