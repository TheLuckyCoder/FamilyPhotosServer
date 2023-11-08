use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Form, Json, Router};
use tracing::{debug, error};

use crate::http::utils::status_error::StatusError;
use crate::http::utils::{AuthSession, AxumResult};
use crate::model::user::{SimpleUser, UserCredentials};

pub fn router() -> Router {
    Router::new()
        .route("/login", post(login))
        .route("/logout", post(logout))
}

async fn login(
    mut auth: AuthSession,
    Form(login_user): Form<UserCredentials>,
) -> AxumResult<impl IntoResponse> {
    let valid_user = auth
        .authenticate(login_user)
        .await
        .map_err(|e| StatusError::create(format!("Failed to validate credentials: {e}")))?;

    let user = match valid_user {
        None => {
            return Err(StatusError::new_status(
                "Wrong user name or password",
                StatusCode::UNAUTHORIZED,
            ))
        }
        Some(user) => user,
    };

    auth.login(&user).await.map_err(|e| {
        error!("Failed to login user with {}: {}", user.id, e);
        StatusError::create("Failed to login")
    })?;

    Ok(Json(SimpleUser::from(user)))
}

async fn logout(mut auth: AuthSession) -> String {
    if let Some(user) = &auth.user {
        debug!("Logging out user: {}", user.id);

        if let Err(e) = auth.logout() {
            return e.to_string();
        }
    }

    "Failed to log out".to_string()
}
