use actix::{Handler, Message};
use diesel::prelude::*;

use crate::model::user::User;
use crate::schema::users::dsl::{id, user_name, users};
use crate::DbActor;

#[derive(Message)]
#[rtype(result = "QueryResult<User>")]
pub struct InsertUser(pub User);

#[derive(Message)]
#[rtype(result = "QueryResult<Vec<User>>")]
pub struct GetUsers;

#[derive(Message)]
#[rtype(result = "QueryResult<User>")]
pub enum GetUser {
    Id(i64),
    UserName(String),
}

impl Handler<InsertUser> for DbActor {
    type Result = QueryResult<User>;

    fn handle(&mut self, msg: InsertUser, _: &mut Self::Context) -> Self::Result {
        let mut conn = self.0.get().expect("Unable to get a connection");

        diesel::insert_into(users)
            .values(msg.0)
            .get_result::<User>(&mut conn)
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
            GetUser::Id(user_id) => users.filter(id.eq(user_id)).get_result::<User>(&mut conn),
            GetUser::UserName(name) => users
                .filter(user_name.eq(name))
                .get_result::<User>(&mut conn),
        }
    }
}
