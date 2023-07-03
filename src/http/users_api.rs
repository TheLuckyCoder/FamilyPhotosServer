use crate::http::status_error::StatusError;
use crate::http::utils::AxumResult;
use crate::model::user::{SimpleUser, User};
use crate::repo::users_repo::UsersRepository;
use crate::utils::password_hash::validate_credentials;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use axum_login::{PostgresStore, RequireAuthorizationLayer};
use serde::Deserialize;
use tracing::{error, warn};

pub type AuthContext = axum_login::extractors::AuthContext<String, User, PostgresStore<User>>;
pub type RequireAuth = RequireAuthorizationLayer<String, User>;

pub fn router(users_repo: UsersRepository) -> Router {
    Router::new()
        .route("/login", post(login))
        .route("/logout", post(logout))
        .with_state(users_repo)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginUser {
    pub user_name: String,
    pub password: String,
}

async fn login(
    State(user_repo): State<UsersRepository>,
    mut auth: AuthContext,
    Query(login_user): Query<LoginUser>,
) -> AxumResult<impl IntoResponse> {
    let user = user_repo.get_user(login_user.user_name).await;

    let valid_credentials = validate_credentials(&login_user.password, &user.password_hash)
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

    auth.login(&user).await.map_err(|e| {
        error!("Failed to login user with {}: {}", user.id, e);
        StatusError::create("Failed to login")
    })?;

    Ok(Json(SimpleUser::from_user(&user)))
}

async fn logout(mut auth: AuthContext) {
    dbg!("Logging out user: {}", &auth.current_user);
    auth.logout().await;
}
