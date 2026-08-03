use crate::models::class::{Class, ClassDetail, ClassFeature, ClassSummary, Subclass, SubclassDetail};
use anyhow::Context;
use sqlx::sqlite::SqlitePool;
use sqlx::Row;

/// Lists every class in the compendium with its subclass count, for the
/// classes index page.
pub async fn list_classes(pool: &SqlitePool) -> anyhow::Result<Vec<ClassSummary>> {
    let rows = sqlx::query(
        "SELECT c.data_json, \
                (SELECT COUNT(*) FROM subclasses s WHERE s.class_id = c.id) AS subclass_count \
         FROM classes c ORDER BY c.name",
    )
    .fetch_all(pool)
    .await
    .context("failed to query classes")?;

    rows.into_iter()
        .map(|row| {
            let data_json: String = row.try_get("data_json").context("missing data_json column")?;
            let class: Class =
                serde_json::from_str(&data_json).context("failed to deserialize class row")?;
            let subclass_count: i64 =
                row.try_get("subclass_count").context("missing subclass_count column")?;
            Ok(ClassSummary {
                id: class.id,
                name: class.name,
                source: class.source,
                hit_die: class.hit_die,
                saving_throws: class.saving_throws,
                subclass_count: subclass_count as u32,
                multiclass_ability_prerequisites: class.multiclass_ability_prerequisites,
            })
        })
        .collect()
}

/// Fetches one class plus all of its features and subclasses (with their
/// features), everything the detail page needs in a single payload.
pub async fn get_class_detail(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<ClassDetail>> {
    let Some(class_row) = sqlx::query("SELECT data_json FROM classes WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .context("failed to query class")?
    else {
        return Ok(None);
    };
    let class_json: String = class_row.try_get("data_json").context("missing data_json column")?;
    let class: Class =
        serde_json::from_str(&class_json).context("failed to deserialize class row")?;

    let features = sqlx::query(
        "SELECT data_json FROM class_features WHERE class_id = ? ORDER BY level, sort_order",
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .context("failed to query class features")?
    .into_iter()
    .map(|row| {
        let data_json: String = row.try_get("data_json").context("missing data_json column")?;
        serde_json::from_str::<ClassFeature>(&data_json)
            .context("failed to deserialize class feature row")
    })
    .collect::<anyhow::Result<Vec<_>>>()?;

    let subclasses = sqlx::query(
        "SELECT data_json FROM subclasses WHERE class_id = ? ORDER BY short_name COLLATE NOCASE",
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .context("failed to query subclasses")?
    .into_iter()
    .map(|row| {
        let data_json: String = row.try_get("data_json").context("missing data_json column")?;
        serde_json::from_str::<Subclass>(&data_json).context("failed to deserialize subclass row")
    })
    .collect::<anyhow::Result<Vec<_>>>()?;

    let mut feature_rows = sqlx::query(
        "SELECT subclass_id, data_json FROM subclass_features \
         WHERE subclass_id IN (SELECT id FROM subclasses WHERE class_id = ?) \
         ORDER BY level, sort_order",
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .context("failed to query subclass features")?
    .into_iter()
    .map(|row| {
        let subclass_id: String =
            row.try_get("subclass_id").context("missing subclass_id column")?;
        let data_json: String = row.try_get("data_json").context("missing data_json column")?;
        let feature = serde_json::from_str::<ClassFeature>(&data_json)
            .context("failed to deserialize subclass feature row")?;
        Ok((subclass_id, feature))
    })
    .collect::<anyhow::Result<Vec<_>>>()?;

    let subclasses = subclasses
        .into_iter()
        .map(|subclass| {
            let features = feature_rows
                .extract_if(.., |(subclass_id, _)| *subclass_id == subclass.id)
                .map(|(_, feature)| feature)
                .collect();
            SubclassDetail { subclass, features }
        })
        .collect();

    Ok(Some(ClassDetail { class, features, subclasses }))
}
