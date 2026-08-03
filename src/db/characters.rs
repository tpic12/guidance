use crate::models::character::{active_subclass, total_level, AsiChoice, Character, CharacterSheet, CharacterSummary};
use crate::models::feat::Feat;
use anyhow::{bail, Context};
use sqlx::sqlite::SqlitePool;
use sqlx::Row;

use super::backgrounds::get_background;
use super::classes::get_class_detail;
use super::get_flat_by_id;
use super::optional_features::get_optional_features_by_ids;
use super::spells::{get_spells_by_ids, get_subclass_granted_spells};

pub async fn list_characters(
    pool: &SqlitePool,
    user_id: &str,
) -> anyhow::Result<Vec<CharacterSummary>> {
    let rows = sqlx::query(
        "SELECT data_json FROM characters WHERE user_id = ? ORDER BY name COLLATE NOCASE",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .context("failed to query characters")?;

    let mut class_names = std::collections::HashMap::new();
    for row in sqlx::query("SELECT id, name FROM classes").fetch_all(pool).await? {
        class_names.insert(row.try_get::<String, _>("id")?, row.try_get::<String, _>("name")?);
    }
    let mut species_names = std::collections::HashMap::new();
    for row in sqlx::query("SELECT id, name FROM species").fetch_all(pool).await? {
        species_names.insert(row.try_get::<String, _>("id")?, row.try_get::<String, _>("name")?);
    }

    rows.into_iter()
        .map(|row| {
            let data_json: String = row.try_get("data_json").context("missing data_json column")?;
            let character: Character = serde_json::from_str(&data_json)
                .context("failed to deserialize character row")?;
            let names: Vec<String> =
                character.classes.iter().filter_map(|entry| class_names.get(&entry.class_id).cloned()).collect();
            Ok(CharacterSummary {
                class_name: (!names.is_empty()).then(|| names.join(" / ")),
                species_name: species_names.get(&character.species_id).cloned(),
                id: character.id,
                name: character.name,
                level: total_level(&character.classes),
            })
        })
        .collect()
}

/// Scoped to `user_id` so a character belonging to another user resolves to
/// `None` — indistinguishable from a nonexistent id, rather than a separate
/// "forbidden" case that would confirm the id exists.
pub async fn get_character(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> anyhow::Result<Option<Character>> {
    let Some(row) = sqlx::query("SELECT data_json FROM characters WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .context("failed to query character by id")?
    else {
        return Ok(None);
    };
    let data_json: String = row.try_get("data_json").context("missing data_json column")?;
    serde_json::from_str(&data_json).map(Some).context("failed to deserialize character row")
}

pub async fn save_character(pool: &SqlitePool, character: &Character) -> anyhow::Result<()> {
    if character.id.is_empty() {
        bail!("character id must be set before saving");
    }
    sqlx::query(
        "INSERT INTO characters (id, name, user_id, data_json, created_at, updated_at) \
         VALUES (?, ?, ?, ?, datetime('now'), datetime('now')) \
         ON CONFLICT(id) DO UPDATE SET \
            name = excluded.name, user_id = excluded.user_id, \
            data_json = excluded.data_json, updated_at = datetime('now')",
    )
    .bind(&character.id)
    .bind(&character.name)
    .bind(&character.user_id)
    .bind(serde_json::to_string(character)?)
    .execute(pool)
    .await
    .context("failed to save character")?;
    Ok(())
}

pub async fn delete_character(pool: &SqlitePool, id: &str, user_id: &str) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM characters WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .context("failed to delete character")?;
    Ok(())
}

pub async fn get_character_sheet(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> anyhow::Result<Option<CharacterSheet>> {
    let Some(character) = get_character(pool, id, user_id).await? else {
        return Ok(None);
    };

    let species = get_flat_by_id(pool, "species", &character.species_id).await?;
    let background = get_background(pool, &character.background_id).await?;

    let mut classes = Vec::new();
    for entry in &character.classes {
        if let Some(detail) = get_class_detail(pool, &entry.class_id).await? {
            classes.push(detail);
        }
    }

    let mut feats = Vec::new();
    for choice in character.asi_choices.iter().flatten() {
        if let AsiChoice::Feat { feat_id } = choice {
            if let Some(feat) = get_flat_by_id::<Feat>(pool, "feats", feat_id).await? {
                feats.push(feat);
            }
        }
    }

    let mut cantrips = get_spells_by_ids(pool, &character.cantrip_choices).await?;
    let mut spells = get_spells_by_ids(pool, &character.spell_choices).await?;
    for spell in get_spells_by_ids(pool, &character.spell_grant_choices).await? {
        if spell.level == 0 { cantrips.push(spell) } else { spells.push(spell) }
    }

    // Every entry's own active subclass (per that class's own level), not
    // just the first class — a purged/archived class simply isn't in
    // `classes` (see `get_class_detail`'s tolerate-dangling behavior above),
    // so it contributes nothing here rather than failing the whole sheet.
    let mut granted_spells = Vec::new();
    for entry in &character.classes {
        let Some(detail) = classes.iter().find(|detail| detail.class.id == entry.class_id) else { continue };
        if let Some(subclass) = active_subclass(detail, entry.subclass_id.as_deref(), entry.level) {
            granted_spells.extend(get_subclass_granted_spells(pool, &subclass.id, entry.level).await?);
        }
    }

    let optional_features = get_optional_features_by_ids(pool, &character.optional_feature_choices).await?;

    Ok(Some(CharacterSheet {
        character,
        species,
        background,
        classes,
        feats,
        cantrips,
        spells,
        granted_spells,
        optional_features,
    }))
}
