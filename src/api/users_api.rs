use actix_web::{get, web::Data, web::Path, HttpResponse, Responder};

use crate::db::users_db::{GetUser, GetUsers};
use crate::model::user::SimpleUser;
use crate::AppState;

// region Public

#[get("")]
pub async fn get_users(state: Data<AppState>) -> impl Responder {
    let db = state.get_ref().db.clone();

    match db.send(GetUsers).await {
        Ok(Ok(users)) => {
            let result = users
                .into_iter()
                .map(|value| SimpleUser::from_user(&value))
                .collect::<Vec<SimpleUser>>();

            HttpResponse::Ok().json(result)
        }
        _ => HttpResponse::InternalServerError().json("Something went wrong"),
    }
}

#[get("/name/{name}")]
pub async fn get_user(state: Data<AppState>, name: Path<String>) -> impl Responder {
    let db = state.get_ref().db.clone();

    match db.send(GetUser::UserName(name.into_inner())).await {
        Ok(Ok(user)) => {
            let result = SimpleUser::from_user(&user);
            HttpResponse::Ok().json(result)
        }
        _ => HttpResponse::InternalServerError().json("Something went wrong"),
    }
}
