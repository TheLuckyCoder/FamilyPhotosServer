use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use axum_login::{PostgresStore, RequireAuthorizationLayer};
use serde::Deserialize;
use tracing::{debug, error};

use crate::http::utils::status_error::StatusError;
use crate::http::utils::AxumResult;
use crate::model::user::{SimpleUser, User};
use crate::repo::users_repo::UsersRepository;
use crate::utils::password_hash::validate_credentials;

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
    pub user_id: String,
    pub password: String,
}

async fn login(
    State(user_repo): State<UsersRepository>,
    mut auth: AuthContext,
    Query(login_user): Query<LoginUser>,
) -> AxumResult<impl IntoResponse> {
    let user = user_repo
        .get_user(login_user.user_id)
        .await
        .ok_or_else(|| {
            StatusError::new_status("Wrong user name or password", StatusCode::UNAUTHORIZED)
        })?;

    let valid_credentials = validate_credentials(&login_user.password, &user.password_hash)
        .map_err(|e| {
            error!("Failed to validate credentials: {e}");
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
    if let Some(user) = &auth.current_user {
        debug!("Logging out user: {}", user.id);

        auth.logout().await;
    }
}
