use crate::models::species::{Species, SpeciesQuery, SpeciesSort};
use sqlx::sqlite::SqlitePool;

use super::{get_flat_by_id, list_distinct_sources, list_flat_compendium};

pub async fn list_species(
    pool: &SqlitePool,
    filter: &SpeciesQuery,
) -> anyhow::Result<Vec<Species>> {
    let order_sql = match filter.sort {
        SpeciesSort::NameAsc => " ORDER BY name ASC",
        SpeciesSort::NameDesc => " ORDER BY name DESC",
        SpeciesSort::SourceAsc => " ORDER BY source ASC, name ASC",
        SpeciesSort::SourceDesc => " ORDER BY source DESC, name ASC",
    };
    list_flat_compendium(pool, "species", &filter.search, &filter.sources, order_sql).await
}

pub async fn list_species_sources(pool: &SqlitePool) -> anyhow::Result<Vec<String>> {
    list_distinct_sources(pool, "species").await
}

pub async fn get_species(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<Species>> {
    get_flat_by_id(pool, "species", id).await
}
