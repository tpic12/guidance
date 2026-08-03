use crate::models::item::{Item, ItemPropertyOption, ItemQuery, ItemSort};
use anyhow::Context;
use sqlx::sqlite::SqlitePool;
use sqlx::{QueryBuilder, Row, Sqlite};

use super::{get_flat_by_id, list_distinct_sources};

/// Lists items matching the given search/filter/sort parameters. Bespoke
/// (not `list_flat_compendium`) for the same reason as
/// `list_optional_features`: several filter dimensions, one of which
/// (`property_codes`) is backed by a join table rather than a plain column.
pub async fn list_items(pool: &SqlitePool, filter: &ItemQuery) -> anyhow::Result<Vec<Item>> {
    let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new("SELECT data_json FROM items");
    let mut where_started = false;

    if !filter.kinds.is_empty() {
        qb.push(" WHERE is_group IN (");
        where_started = true;
        let mut separated = qb.separated(", ");
        for kind in &filter.kinds {
            separated.push_bind(matches!(kind, crate::models::item::ItemKind::Group) as i64);
        }
        separated.push_unseparated(")");
    }

    if !filter.types.is_empty() {
        qb.push(if where_started { " AND " } else { " WHERE " });
        where_started = true;
        qb.push("item_type IN (");
        let mut separated = qb.separated(", ");
        for item_type in &filter.types {
            separated.push_bind(item_type);
        }
        separated.push_unseparated(")");
    }

    if !filter.rarities.is_empty() {
        qb.push(if where_started { " AND " } else { " WHERE " });
        where_started = true;
        qb.push("rarity IN (");
        let mut separated = qb.separated(", ");
        for rarity in &filter.rarities {
            separated.push_bind(rarity);
        }
        separated.push_unseparated(")");
    }

    if !filter.weapon_categories.is_empty() {
        qb.push(if where_started { " AND " } else { " WHERE " });
        where_started = true;
        qb.push("weapon_category IN (");
        let mut separated = qb.separated(", ");
        for category in &filter.weapon_categories {
            separated.push_bind(category.as_str());
        }
        separated.push_unseparated(")");
    }

    if !filter.damage_types.is_empty() {
        qb.push(if where_started { " AND " } else { " WHERE " });
        where_started = true;
        qb.push("damage_type IN (");
        let mut separated = qb.separated(", ");
        for damage_type in &filter.damage_types {
            separated.push_bind(damage_type.as_str());
        }
        separated.push_unseparated(")");
    }

    if !filter.attunement.is_empty() {
        qb.push(if where_started { " AND " } else { " WHERE " });
        where_started = true;
        qb.push("requires_attunement IN (");
        let mut separated = qb.separated(", ");
        for attunement in &filter.attunement {
            separated.push_bind(attunement.as_bool() as i64);
        }
        separated.push_unseparated(")");
    }

    if !filter.property_codes.is_empty() {
        qb.push(if where_started { " AND " } else { " WHERE " });
        where_started = true;
        qb.push("id IN (SELECT item_id FROM item_properties WHERE property_code IN (");
        let mut separated = qb.separated(", ");
        for code in &filter.property_codes {
            separated.push_bind(code);
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
        ItemSort::NameAsc => " ORDER BY name ASC",
        ItemSort::NameDesc => " ORDER BY name DESC",
        ItemSort::SourceAsc => " ORDER BY source ASC, name ASC",
        ItemSort::SourceDesc => " ORDER BY source DESC, name ASC",
        ItemSort::RarityAsc => " ORDER BY rarity_rank ASC, name ASC",
        ItemSort::RarityDesc => " ORDER BY rarity_rank DESC, name ASC",
        ItemSort::ValueAsc => " ORDER BY value_cp IS NULL, value_cp ASC, name ASC",
        ItemSort::ValueDesc => " ORDER BY value_cp IS NULL, value_cp DESC, name ASC",
        ItemSort::WeightAsc => " ORDER BY weight_lb IS NULL, weight_lb ASC, name ASC",
        ItemSort::WeightDesc => " ORDER BY weight_lb IS NULL, weight_lb DESC, name ASC",
    });

    let rows = qb.build().fetch_all(pool).await.context("failed to query items")?;

    rows.into_iter()
        .map(|row| {
            let data_json: String = row.try_get("data_json").context("missing data_json column")?;
            serde_json::from_str::<Item>(&data_json).context("failed to deserialize item row")
        })
        .collect()
}

pub async fn list_item_sources(pool: &SqlitePool) -> anyhow::Result<Vec<String>> {
    list_distinct_sources(pool, "items").await
}

pub async fn list_item_types(pool: &SqlitePool) -> anyhow::Result<Vec<String>> {
    let rows = sqlx::query("SELECT DISTINCT item_type FROM items WHERE item_type IS NOT NULL ORDER BY item_type")
        .fetch_all(pool)
        .await
        .context("failed to query item types")?;

    rows.into_iter()
        .map(|row| row.try_get::<String, _>("item_type").context("missing item_type column"))
        .collect()
}

/// Ordered by severity (`rarity_rank`), not alphabetically, so the filter
/// chip row reads "None, Common, Uncommon, ..., Artifact".
pub async fn list_item_rarities(pool: &SqlitePool) -> anyhow::Result<Vec<String>> {
    let rows = sqlx::query("SELECT rarity FROM items GROUP BY rarity ORDER BY MIN(rarity_rank)")
        .fetch_all(pool)
        .await
        .context("failed to query item rarities")?;

    rows.into_iter()
        .map(|row| row.try_get::<String, _>("rarity").context("missing rarity column"))
        .collect()
}

pub async fn list_item_properties(pool: &SqlitePool) -> anyhow::Result<Vec<ItemPropertyOption>> {
    let rows = sqlx::query("SELECT DISTINCT property_code, property_label FROM item_properties ORDER BY property_label")
        .fetch_all(pool)
        .await
        .context("failed to query item properties")?;

    rows.into_iter()
        .map(|row| {
            Ok(ItemPropertyOption {
                code: row.try_get("property_code").context("missing property_code column")?,
                label: row.try_get("property_label").context("missing property_label column")?,
            })
        })
        .collect()
}

pub async fn get_item(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<Item>> {
    get_flat_by_id(pool, "items", id).await
}

/// Items whose raw type abbreviation matches `code` (e.g. "AT" for artisan's
/// tools) — used to resolve a `ToolCategory`'s real members at read time
/// instead of a hardcoded list. Not backed by an indexed column since
/// `item_type` stores the resolved display label, not the raw code.
pub async fn list_items_by_type_code(pool: &SqlitePool, code: &str) -> anyhow::Result<Vec<Item>> {
    let rows = sqlx::query(
        "SELECT data_json FROM items WHERE json_extract(data_json, '$.item_type_code') = ? ORDER BY name",
    )
    .bind(code)
    .fetch_all(pool)
    .await
    .context("failed to query items by type code")?;

    rows.into_iter()
        .map(|row| {
            let data_json: String = row.try_get("data_json").context("missing data_json column")?;
            serde_json::from_str::<Item>(&data_json).context("failed to deserialize item row")
        })
        .collect()
}
