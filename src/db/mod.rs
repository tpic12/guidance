// db_tests.rs relies on `use super::*` to reach these — keep the set even
// where mod.rs's own code doesn't use every name.
#[allow(unused_imports)]
use crate::models::background::{Background, BackgroundQuery, BackgroundSort};
#[allow(unused_imports)]
use crate::models::character::{active_subclass, total_level, AsiChoice, Character, CharacterSheet, CharacterSummary};
#[allow(unused_imports)]
use crate::models::class::{Class, ClassDetail, ClassFeature, ClassSummary, Subclass, SubclassDetail};
#[allow(unused_imports)]
use crate::models::feat::{Feat, FeatQuery, FeatSort};
#[allow(unused_imports)]
use crate::models::item::{Item, ItemPropertyOption, ItemQuery, ItemSort};
#[allow(unused_imports)]
use crate::models::optional_feature::{OptionalFeature, OptionalFeatureQuery, OptionalFeatureSort};
#[allow(unused_imports)]
use crate::models::species::{Species, SpeciesQuery, SpeciesSort};
#[allow(unused_imports)]
use crate::models::spell::{Spell, SpellQuery, SpellSort};
#[allow(unused_imports)]
use crate::models::user::{CurrentUser, Permission, UserSummary};
use anyhow::{bail, Context};
#[allow(unused_imports)]
use argon2::password_hash::{rand_core::OsRng, SaltString};
#[allow(unused_imports)]
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use sqlx::migrate::MigrateDatabase;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions};
use sqlx::{QueryBuilder, Row, Sqlite};
use std::str::FromStr;

mod auth;
mod backgrounds;
mod characters;
mod classes;
mod feats;
mod items;
mod languages;
mod optional_features;
mod species;
mod spells;

pub use auth::*;
pub use backgrounds::*;
pub use characters::*;
pub use classes::*;
pub use feats::*;
pub use items::*;
pub use languages::*;
pub use optional_features::*;
pub use species::*;
pub use spells::*;

const DEFAULT_DEV_DB_URL: &str = "sqlite://dev.db";

/// Connects to the local sqlite dev database, creating the file and running
/// migrations if needed. Refuses to run against a non-sqlite DATABASE_URL so
/// this can't accidentally be pointed at a production database.
pub async fn connect_dev_db() -> anyhow::Result<SqlitePool> {
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DEV_DB_URL.to_string());
    if !db_url.starts_with("sqlite://") {
        bail!("connect_dev_db only supports local sqlite dev databases, got DATABASE_URL={db_url}");
    }

    if !Sqlite::database_exists(&db_url).await.unwrap_or(false) {
        Sqlite::create_database(&db_url)
            .await
            .with_context(|| format!("failed to create dev database at {db_url}"))?;
    }

    // sqlx defaults to WAL, which coordinates readers/writers through a
    // shared-memory index (the `-shm` file). That index isn't reliably
    // coherent across separate processes when the db file lives on a
    // bind-mounted/virtiofs path (e.g. the Docker dev container), which
    // surfaces as a `disk I/O error` the moment a second process (like a
    // `seed` run) opens the db while the server is still running. The
    // classic rollback journal uses plain POSIX file locks instead, which
    // bind mounts handle correctly, at the cost of writers briefly blocking
    // readers instead of allowing full WAL concurrency — an acceptable
    // trade for a local dev/CLI-seed database.
    let connect_options = SqliteConnectOptions::from_str(&db_url)
        .with_context(|| format!("invalid DATABASE_URL={db_url}"))?
        .journal_mode(SqliteJournalMode::Delete);

    let pool = SqlitePoolOptions::new()
        .connect_with(connect_options)
        .await
        .with_context(|| format!("failed to connect to dev database at {db_url}"))?;

    sqlx::migrate!("./migrations").run(&pool).await?;
    bootstrap_admin_if_empty(&pool).await?;

    Ok(pool)
}

/// Shared list query for the flat compendium tables (backgrounds, feats, species).
async fn list_flat_compendium<T: serde::de::DeserializeOwned>(
    pool: &SqlitePool,
    table: &'static str,
    search: &str,
    sources: &[String],
    order_sql: &'static str,
) -> anyhow::Result<Vec<T>> {
    let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new(format!("SELECT data_json FROM {table}"));
    let mut where_started = false;

    if !sources.is_empty() {
        qb.push(" WHERE source IN (");
        where_started = true;
        let mut separated = qb.separated(", ");
        for source in sources {
            separated.push_bind(source);
        }
        separated.push_unseparated(")");
    }

    let search = search.trim();
    if !search.is_empty() {
        qb.push(if where_started { " AND " } else { " WHERE " });
        qb.push("name LIKE ").push_bind(format!("%{search}%"));
    }

    qb.push(order_sql);

    let rows = qb
        .build()
        .fetch_all(pool)
        .await
        .with_context(|| format!("failed to query {table}"))?;

    rows.into_iter()
        .map(|row| {
            let data_json: String =
                row.try_get("data_json").context("missing data_json column")?;
            serde_json::from_str::<T>(&data_json)
                .with_context(|| format!("failed to deserialize {table} row"))
        })
        .collect()
}

async fn list_distinct_sources(pool: &SqlitePool, table: &'static str) -> anyhow::Result<Vec<String>> {
    let rows = sqlx::query(&format!("SELECT DISTINCT source FROM {table} ORDER BY source"))
        .fetch_all(pool)
        .await
        .with_context(|| format!("failed to query {table} sources"))?;

    rows.into_iter()
        .map(|row| row.try_get::<String, _>("source").context("missing source column"))
        .collect()
}

async fn get_flat_by_id<T: serde::de::DeserializeOwned>(
    pool: &SqlitePool,
    table: &'static str,
    id: &str,
) -> anyhow::Result<Option<T>> {
    let Some(row) = sqlx::query(&format!("SELECT data_json FROM {table} WHERE id = ?"))
        .bind(id)
        .fetch_optional(pool)
        .await
        .with_context(|| format!("failed to query {table} by id"))?
    else {
        return Ok(None);
    };
    let data_json: String = row.try_get("data_json").context("missing data_json column")?;
    serde_json::from_str(&data_json)
        .map(Some)
        .with_context(|| format!("failed to deserialize {table} row"))
}

#[cfg(test)]
#[path = "db_tests.rs"]
mod tests;

