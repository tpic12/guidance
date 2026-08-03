use crate::models::feat::{Feat, FeatQuery, FeatSort};
use sqlx::sqlite::SqlitePool;

use super::{list_distinct_sources, list_flat_compendium};

pub async fn list_feats(pool: &SqlitePool, filter: &FeatQuery) -> anyhow::Result<Vec<Feat>> {
    let order_sql = match filter.sort {
        FeatSort::NameAsc => " ORDER BY name ASC",
        FeatSort::NameDesc => " ORDER BY name DESC",
        FeatSort::SourceAsc => " ORDER BY source ASC, name ASC",
        FeatSort::SourceDesc => " ORDER BY source DESC, name ASC",
    };
    list_flat_compendium(pool, "feats", &filter.search, &filter.sources, order_sql).await
}

pub async fn list_feat_sources(pool: &SqlitePool) -> anyhow::Result<Vec<String>> {
    list_distinct_sources(pool, "feats").await
}
