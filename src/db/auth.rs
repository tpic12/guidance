use crate::models::user::{CurrentUser, Permission, UserSummary};
use anyhow::Context;
use argon2::password_hash::{rand_core::OsRng, SaltString};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use sqlx::sqlite::SqlitePool;
use sqlx::Row;

/// So every fresh Docker Compose deploy has a working login with no manual
/// setup step. Safe to call on every boot: a no-op once any user exists.
pub async fn bootstrap_admin_if_empty(pool: &SqlitePool) -> anyhow::Result<()> {
    let row = sqlx::query("SELECT COUNT(*) AS count FROM users")
        .fetch_one(pool)
        .await
        .context("failed to count users")?;
    let count: i64 = row.try_get("count").context("missing count column")?;
    if count > 0 {
        return Ok(());
    }
    create_user(pool, "admin", "admin", true)
        .await
        .context("failed to bootstrap admin user")?;
    Ok(())
}

fn hash_password(password: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|err| anyhow::anyhow!("failed to hash password: {err}"))
}

fn verify_password(password: &str, hash: &str) -> anyhow::Result<bool> {
    let parsed =
        PasswordHash::new(hash).map_err(|err| anyhow::anyhow!("corrupt password hash: {err}"))?;
    Ok(Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok())
}

fn permission_from_str(value: &str) -> anyhow::Result<Permission> {
    Permission::ALL
        .into_iter()
        .find(|permission| permission.as_str() == value)
        .ok_or_else(|| anyhow::anyhow!("unknown permission: {value}"))
}

fn map_unique_violation(err: sqlx::Error, message: &'static str) -> anyhow::Error {
    match &err {
        sqlx::Error::Database(db_err) if db_err.is_unique_violation() => anyhow::anyhow!(message),
        _ => anyhow::Error::from(err).context(message),
    }
}

/// `is_instance_owner` must only ever be `true` for the bootstrap admin
/// account created by `bootstrap_admin_if_empty` — there is deliberately no
/// function anywhere in this module to flip it after creation, since its
/// permanence is what guarantees the instance can never be locked out of
/// admin access.
pub async fn create_user(
    pool: &SqlitePool,
    username: &str,
    password: &str,
    is_instance_owner: bool,
) -> anyhow::Result<String> {
    let id = uuid::Uuid::new_v4().to_string();
    let password_hash = hash_password(password)?;
    sqlx::query(
        "INSERT INTO users (id, username, password_hash, is_instance_owner, theme) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(username)
    .bind(&password_hash)
    .bind(is_instance_owner)
    .bind("guidance")
    .execute(pool)
    .await
    .map_err(|err| map_unique_violation(err, "username already taken"))?;
    Ok(id)
}

pub async fn authenticate(
    pool: &SqlitePool,
    username: &str,
    password: &str,
) -> anyhow::Result<Option<String>> {
    let Some(row) = sqlx::query("SELECT id, password_hash FROM users WHERE username = ?")
        .bind(username)
        .fetch_optional(pool)
        .await
        .context("failed to query user by username")?
    else {
        return Ok(None);
    };

    let id: String = row.try_get("id").context("missing id column")?;
    let password_hash: String =
        row.try_get("password_hash").context("missing password_hash column")?;

    if verify_password(password, &password_hash)? {
        Ok(Some(id))
    } else {
        Ok(None)
    }
}

async fn list_permissions_for_user(
    pool: &SqlitePool,
    user_id: &str,
) -> anyhow::Result<Vec<Permission>> {
    sqlx::query("SELECT permission FROM user_permissions WHERE user_id = ?")
        .bind(user_id)
        .fetch_all(pool)
        .await
        .context("failed to query user permissions")?
        .into_iter()
        .map(|row| {
            let permission: String =
                row.try_get("permission").context("missing permission column")?;
            permission_from_str(&permission)
        })
        .collect()
}

pub async fn get_current_user(
    pool: &SqlitePool,
    user_id: &str,
) -> anyhow::Result<Option<CurrentUser>> {
    let Some(row) =
        sqlx::query("SELECT id, username, is_instance_owner, theme FROM users WHERE id = ?")
            .bind(user_id)
            .fetch_optional(pool)
            .await
            .context("failed to query user")?
    else {
        return Ok(None);
    };

    let id: String = row.try_get("id").context("missing id column")?;
    let permissions = list_permissions_for_user(pool, &id).await?;

    Ok(Some(CurrentUser {
        id,
        username: row.try_get("username").context("missing username column")?,
        is_instance_owner: row
            .try_get("is_instance_owner")
            .context("missing is_instance_owner column")?,
        permissions,
        theme: row.try_get("theme").context("missing theme column")?,
    }))
}

pub async fn list_users(pool: &SqlitePool) -> anyhow::Result<Vec<UserSummary>> {
    let rows = sqlx::query("SELECT id, username, is_instance_owner FROM users ORDER BY username")
        .fetch_all(pool)
        .await
        .context("failed to query users")?;

    let mut users = Vec::with_capacity(rows.len());
    for row in rows {
        let id: String = row.try_get("id").context("missing id column")?;
        let username: String = row.try_get("username").context("missing username column")?;
        let is_instance_owner: bool = row
            .try_get("is_instance_owner")
            .context("missing is_instance_owner column")?;
        let permissions = list_permissions_for_user(pool, &id).await?;
        users.push(UserSummary { id, username, is_instance_owner, permissions });
    }
    Ok(users)
}

pub async fn update_username(
    pool: &SqlitePool,
    user_id: &str,
    new_username: &str,
) -> anyhow::Result<()> {
    sqlx::query("UPDATE users SET username = ? WHERE id = ?")
        .bind(new_username)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|err| map_unique_violation(err, "username already taken"))?;
    Ok(())
}

pub async fn update_password(
    pool: &SqlitePool,
    user_id: &str,
    new_password: &str,
) -> anyhow::Result<()> {
    let password_hash = hash_password(new_password)?;
    sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
        .bind(password_hash)
        .bind(user_id)
        .execute(pool)
        .await
        .context("failed to update password")?;
    Ok(())
}

/// Caller must validate `theme` against `models::user::THEMES` first — this
/// just persists whatever string it's given.
pub async fn update_theme(pool: &SqlitePool, user_id: &str, theme: &str) -> anyhow::Result<()> {
    sqlx::query("UPDATE users SET theme = ? WHERE id = ?")
        .bind(theme)
        .bind(user_id)
        .execute(pool)
        .await
        .context("failed to update theme")?;
    Ok(())
}

/// Re-granting a permission a user already has is a no-op, not an error.
pub async fn grant_permission(
    pool: &SqlitePool,
    user_id: &str,
    permission: Permission,
    granted_by: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT OR IGNORE INTO user_permissions (user_id, permission, granted_by) VALUES (?, ?, ?)",
    )
    .bind(user_id)
    .bind(permission.as_str())
    .bind(granted_by)
    .execute(pool)
    .await
    .context("failed to grant permission")?;
    Ok(())
}

pub async fn revoke_permission(
    pool: &SqlitePool,
    user_id: &str,
    permission: Permission,
) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM user_permissions WHERE user_id = ? AND permission = ?")
        .bind(user_id)
        .bind(permission.as_str())
        .execute(pool)
        .await
        .context("failed to revoke permission")?;
    Ok(())
}
