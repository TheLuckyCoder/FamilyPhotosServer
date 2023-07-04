use async_trait::async_trait;
use axum_login::axum_sessions::async_session::{Result, Session, SessionStore};
use sqlx::{query, query_as, PgPool};
use tracing::trace;

#[derive(Debug, Clone)]
pub struct PgSessionRepository {
    pool: PgPool,
}

struct DatabaseSession {
    pub session: String,
    pub cookie: Option<String>,
}

#[async_trait]
impl SessionStore for PgSessionRepository {
    async fn load_session(&self, cookie_value: String) -> Result<Option<Session>> {
        let id = Session::id_from_cookie_value(&cookie_value)?;
        trace!("loading session by id `{}`", id);

        let db_session_opt = query_as!(
            DatabaseSession,
            "select session, cookie from session where key = $1",
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(db_session) = db_session_opt {
            let mut session = serde_json::from_str::<Session>(db_session.session.as_str())?;

            if let Some(cookie) = db_session.cookie {
                session.set_cookie_value(cookie);
            }

            return Ok(session.validate());
        }

        Ok(None)
    }

    async fn store_session(&self, session: Session) -> Result<Option<String>> {
        trace!("storing session by id `{}`", session.id());

        query!(
            "insert into session (key, session, cookie) values ($1, $2, $3) on conflict (key) do update set session = $2, cookie = $3",
            session.id().to_string(),
            serde_json::to_string(&session)?,
            session.clone().into_cookie_value() // TODO
        )
        .execute(&self.pool)
        .await?;

        session.reset_data_changed();
        Ok(session.into_cookie_value())
    }

    async fn destroy_session(&self, session: Session) -> Result {
        trace!("destroying session by id `{}`", session.id());

        query!(
            "delete from session where key = $1",
            session.id().to_string()
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn clear_store(&self) -> Result {
        trace!("clearing memory store");

        query!("delete from session").execute(&self.pool).await?;
        Ok(())
    }
}

impl PgSessionRepository {
    /// Create a new instance of MemoryStore
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
