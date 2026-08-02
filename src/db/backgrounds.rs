use crate::models::background::{Background, BackgroundQuery, BackgroundSort};
use sqlx::sqlite::SqlitePool;

use super::{get_flat_by_id, list_distinct_sources, list_flat_compendium};

pub async fn list_backgrounds(
    pool: &SqlitePool,
    filter: &BackgroundQuery,
) -> anyhow::Result<Vec<Background>> {
    let order_sql = match filter.sort {
        BackgroundSort::NameAsc => " ORDER BY name ASC",
        BackgroundSort::NameDesc => " ORDER BY name DESC",
        BackgroundSort::SourceAsc => " ORDER BY source ASC, name ASC",
        BackgroundSort::SourceDesc => " ORDER BY source DESC, name ASC",
    };
    list_flat_compendium(pool, "backgrounds", &filter.search, &filter.sources, order_sql).await
}

pub async fn list_background_sources(pool: &SqlitePool) -> anyhow::Result<Vec<String>> {
    list_distinct_sources(pool, "backgrounds").await
}

pub async fn get_background(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<Background>> {
    get_flat_by_id(pool, "backgrounds", id).await
}
