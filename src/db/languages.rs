use crate::models::language::Language;
use sqlx::sqlite::SqlitePool;

use super::list_flat_compendium;

pub async fn list_languages(pool: &SqlitePool) -> anyhow::Result<Vec<Language>> {
    list_flat_compendium(pool, "languages", "", &[], " ORDER BY name ASC").await
}
