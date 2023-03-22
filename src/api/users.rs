use actix_web::{get, post, web, web::Data, web::Path, HttpResponse, Responder};

use crate::db::users::{GetUser, GetUsers, InsertUser};
use crate::model::user::{SimpleUser, User};
use crate::utils::password_hash::get_hash_from_password;
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

#[post("")]
pub async fn create_user(state: Data<AppState>, user: web::Json<User>) -> impl Responder {
    let db = state.get_ref().db.clone();

    let mut hashed_user = user.into_inner();
    hashed_user.password = get_hash_from_password(&hashed_user.password);

    match db.send(InsertUser(hashed_user)).await {
        Ok(Ok(user)) => {
            let result = SimpleUser::from_user(&user);
            HttpResponse::Ok().json(result)
        }
        _ => HttpResponse::InternalServerError().json("Something went wrong"),
    }
}
