use crate::db::users_db::{GetUser, GetUsers};
use crate::db::{internal_error, Handler, Pool};
use crate::http::status_error::StatusError;
use crate::http::AxumResult;
use crate::model::user::SimpleUser;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};

pub fn router(pool: Pool) -> Router {
    Router::new()
        .route("/list", get(list_users))
        .route("/get/:name", get(get_user))
        .with_state(pool)
}

async fn list_users(State(pool): State<Pool>) -> AxumResult<impl IntoResponse> {
    let users = pool.send(GetUsers).await.map_err(internal_error)?;

    let result = users
        .into_iter()
        .map(|value| SimpleUser::from_user(&value))
        .collect::<Vec<SimpleUser>>();

    Ok(Json(result))
}

async fn get_user(
    State(pool): State<Pool>,
    Path(name): Path<String>,
) -> AxumResult<impl IntoResponse> {
    let user = pool
        .send(GetUser::UserName(name))
        .await
        .map_err(|_| StatusError::new_status("No such user", StatusCode::NOT_FOUND))?;

    Ok(Json(SimpleUser::from_user(&user)))
}
