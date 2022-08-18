use actix_web::{get, HttpResponse, Responder};

#[get("/ping")]
pub async fn ping() -> impl Responder {
    HttpResponse::Ok()
}