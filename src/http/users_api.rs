use crate::http::status_error::StatusError;
use crate::http::utils::AxumResult;
use crate::model::user::{SimpleUser, User};
use crate::repo::users_repo::UsersRepository;
use crate::utils::internal_error;
use crate::utils::password_hash::validate_credentials;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use axum_login::{PostgresStore, RequireAuthorizationLayer};
use serde::Deserialize;
use tracing::warn;

pub type AuthContext = axum_login::extractors::AuthContext<String, User, PostgresStore<User>>;
pub type RequireAuth = RequireAuthorizationLayer<String, User>;

pub fn router(users_repo: UsersRepository) -> Router {
    let protected_router = Router::new()
        .route("/list", get(list_users))
        // .route("/get/:name", get(get_user))
        .route_layer(RequireAuth::login())
        .with_state(users_repo.clone());

    Router::new()
        .route("/login", post(login))
        .route("/logout", post(logout))
        .merge(protected_router)
        .with_state(users_repo)
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginUser {
    pub user_name: String,
    pub password: String,
}

async fn login(
    State(user_repo): State<UsersRepository>,
    mut auth: AuthContext,
    Path(login_user): Path<LoginUser>,
) -> AxumResult<impl IntoResponse> {
    let user = user_repo.get_user(login_user.user_name).await;

    let valid_credentials = validate_credentials(&user.password_hash, &login_user.password)
        .map_err(|e| {
            warn!("Failed to validate credentials: {e}");
            StatusError::create("Failed to validate credentials")
        })?;

    if !valid_credentials {
        return Err(StatusError::new_status(
            "Wrong user name or password",
            StatusCode::UNAUTHORIZED,
        ));
    }

    auth.login(&user).await.unwrap();

    Ok(Json(SimpleUser::from_user(&user)))
}

async fn logout(mut auth: AuthContext) {
    dbg!("Logging out user: {}", &auth.current_user);
    auth.logout().await;
}

async fn list_users(State(user_repo): State<UsersRepository>) -> AxumResult<impl IntoResponse> {
    let users = user_repo.get_users().await.map_err(internal_error)?;

    let result = users
        .into_iter()
        .map(|value| SimpleUser::from_user(&value))
        .collect::<Vec<SimpleUser>>();

    Ok(Json(result))
}

/*async fn get_user(
    State(app_state): State<AppState>,
    Path(name): Path<String>,
) -> AxumResult<impl IntoResponse> {
    let user = app_state
        .pool
        .send(GetUser::UserName(name))
        .await
        .map_err(|_| StatusError::new_status("No such user", StatusCode::NOT_FOUND))?;

    Ok(Json(SimpleUser::from_user(&user)))
}*/
