use crate::models::user::{CurrentUser, Permission};
use leptos::prelude::ServerFnError;
use sqlx::SqlitePool;
use tower_sessions::Session;

const SESSION_USER_ID_KEY: &str = "user_id";

pub async fn login(session: &Session, user_id: String) -> Result<(), ServerFnError> {
    session
        .insert(SESSION_USER_ID_KEY, user_id)
        .await
        .map_err(|err| ServerFnError::new(err.to_string()))
}

pub async fn logout(session: &Session) -> Result<(), ServerFnError> {
    session.flush().await.map_err(|err| ServerFnError::new(err.to_string()))
}

/// Resolves the current session into a full `CurrentUser`, re-reading
/// permissions from the DB on every call so a permission change/revoke by an
/// admin takes effect immediately rather than waiting on session expiry.
pub async fn current_user(
    pool: &SqlitePool,
    session: &Session,
) -> Result<Option<CurrentUser>, ServerFnError> {
    let Some(user_id) = session
        .get::<String>(SESSION_USER_ID_KEY)
        .await
        .map_err(|err| ServerFnError::new(err.to_string()))?
    else {
        return Ok(None);
    };

    crate::db::get_current_user(pool, &user_id)
        .await
        .map_err(|err| ServerFnError::new(err.to_string()))
}

pub async fn require_login(
    pool: &SqlitePool,
    session: &Session,
) -> Result<CurrentUser, ServerFnError> {
    current_user(pool, session)
        .await?
        .ok_or_else(|| ServerFnError::new("not logged in"))
}

pub async fn require_permission(
    pool: &SqlitePool,
    session: &Session,
    permission: Permission,
) -> Result<CurrentUser, ServerFnError> {
    let user = require_login(pool, session).await?;
    if !user.has_permission(permission) {
        return Err(ServerFnError::new("forbidden"));
    }
    Ok(user)
}
