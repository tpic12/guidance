use crate::models::optional_feature::{OptionalFeature, OptionalFeatureQuery, OptionalFeatureSort};
use anyhow::Context;
use sqlx::sqlite::SqlitePool;
use sqlx::{QueryBuilder, Row, Sqlite};

use super::list_distinct_sources;

/// Lists optional features matching the given search/filter/sort
/// parameters. Bespoke (not `list_flat_compendium`) because `types` filters
/// on `optional_feature_types`, a separate join table — a feature can carry
/// more than one `feature_type` (Fighting Styles are shared across several
/// classes), so it can't live as a single column on `optional_features`
/// itself the way `source`/`name` do.
pub async fn list_optional_features(
    pool: &SqlitePool,
    filter: &OptionalFeatureQuery,
) -> anyhow::Result<Vec<OptionalFeature>> {
    let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new("SELECT data_json FROM optional_features");
    let mut where_started = false;

    if !filter.types.is_empty() {
        qb.push(" WHERE id IN (SELECT optional_feature_id FROM optional_feature_types WHERE feature_type IN (");
        where_started = true;
        let mut separated = qb.separated(", ");
        for feature_type in &filter.types {
            separated.push_bind(feature_type.as_str());
        }
        separated.push_unseparated("))");
    }

    if !filter.class_requirements.is_empty() {
        qb.push(if where_started { " AND " } else { " WHERE " });
        where_started = true;
        qb.push(
            "id IN (SELECT optional_feature_id FROM optional_feature_prerequisite_classes WHERE class_requirement IN (",
        );
        let mut separated = qb.separated(", ");
        for class_requirement in &filter.class_requirements {
            separated.push_bind(class_requirement);
        }
        separated.push_unseparated("))");
    }

    if !filter.pacts.is_empty() {
        qb.push(if where_started { " AND " } else { " WHERE " });
        where_started = true;
        qb.push("id IN (SELECT optional_feature_id FROM optional_feature_prerequisite_pacts WHERE pact IN (");
        let mut separated = qb.separated(", ");
        for pact in &filter.pacts {
            separated.push_bind(pact);
        }
        separated.push_unseparated("))");
    }

    if !filter.sources.is_empty() {
        qb.push(if where_started { " AND " } else { " WHERE " });
        where_started = true;
        qb.push("source IN (");
        let mut separated = qb.separated(", ");
        for source in &filter.sources {
            separated.push_bind(source);
        }
        separated.push_unseparated(")");
    }

    let search = filter.search.trim();
    if !search.is_empty() {
        qb.push(if where_started { " AND " } else { " WHERE " });
        qb.push("name LIKE ").push_bind(format!("%{search}%"));
    }

    qb.push(match filter.sort {
        OptionalFeatureSort::NameAsc => " ORDER BY name ASC",
        OptionalFeatureSort::NameDesc => " ORDER BY name DESC",
        OptionalFeatureSort::SourceAsc => " ORDER BY source ASC, name ASC",
        OptionalFeatureSort::SourceDesc => " ORDER BY source DESC, name ASC",
    });

    let rows = qb.build().fetch_all(pool).await.context("failed to query optional features")?;

    rows.into_iter()
        .map(|row| {
            let data_json: String = row.try_get("data_json").context("missing data_json column")?;
            serde_json::from_str::<OptionalFeature>(&data_json)
                .context("failed to deserialize optional feature row")
        })
        .collect()
}

pub async fn list_optional_feature_sources(pool: &SqlitePool) -> anyhow::Result<Vec<String>> {
    list_distinct_sources(pool, "optional_features").await
}

pub async fn list_optional_feature_class_requirements(pool: &SqlitePool) -> anyhow::Result<Vec<String>> {
    let rows = sqlx::query(
        "SELECT DISTINCT class_requirement FROM optional_feature_prerequisite_classes ORDER BY class_requirement",
    )
    .fetch_all(pool)
    .await
    .context("failed to query optional feature class requirements")?;

    rows.into_iter()
        .map(|row| row.try_get::<String, _>("class_requirement").context("missing class_requirement column"))
        .collect()
}

pub async fn list_optional_feature_pacts(pool: &SqlitePool) -> anyhow::Result<Vec<String>> {
    let rows = sqlx::query("SELECT DISTINCT pact FROM optional_feature_prerequisite_pacts ORDER BY pact")
        .fetch_all(pool)
        .await
        .context("failed to query optional feature pacts")?;

    rows.into_iter().map(|row| row.try_get::<String, _>("pact").context("missing pact column")).collect()
}

/// Resolves optional-feature ids back into full records, tolerating ids
/// that no longer exist — same tolerate-dangling policy as
/// `get_spells_by_ids`.
pub async fn get_optional_features_by_ids(
    pool: &SqlitePool,
    ids: &[String],
) -> anyhow::Result<Vec<OptionalFeature>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new("SELECT data_json FROM optional_features WHERE id IN (");
    let mut separated = qb.separated(", ");
    for id in ids {
        separated.push_bind(id);
    }
    separated.push_unseparated(")");

    let rows = qb.build().fetch_all(pool).await.context("failed to query optional features by id")?;

    rows.into_iter()
        .map(|row| {
            let data_json: String = row.try_get("data_json").context("missing data_json column")?;
            serde_json::from_str::<OptionalFeature>(&data_json)
                .context("failed to deserialize optional feature row")
        })
        .collect()
}
