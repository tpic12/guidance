use anyhow::bail;
use guidance::models::optional_feature::{class_requirement_labels, required_pacts};
use guidance::{db::connect_dev_db, importer};
use sqlx::{Row, SqlitePool};

const DEFAULT_DEV_DB_URL: &str = "sqlite://dev.db";
const MOCK_FIXTURES_DIR: &str = "test-fixtures";

// Guards against accidentally seeding invented e2e/CI records (Fake Bolt,
// Fake Warrior, ...) into a real dev database, where they'd sit alongside
// (or overwrite) actual imported content with no easy way to tell them apart.
fn is_unsafe_mock_seed(fixtures_dir: &str, db_url: &str) -> bool {
    fixtures_dir == MOCK_FIXTURES_DIR && db_url == DEFAULT_DEV_DB_URL
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // `fixtures/` (untracked, user-imported) by default; CI/e2e set `test-fixtures/`.
    let fixtures_dir = std::env::var("FIXTURES_DIR").unwrap_or_else(|_| "fixtures".to_string());
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DEV_DB_URL.to_string());

    if is_unsafe_mock_seed(&fixtures_dir, &db_url) {
        bail!(
            "refusing to seed mock data from `{MOCK_FIXTURES_DIR}/` into the default dev database ({DEFAULT_DEV_DB_URL}). \
             Point DATABASE_URL at a dedicated e2e database instead, e.g. `DATABASE_URL=sqlite://e2e.db`."
        );
    }

    let pool = connect_dev_db().await?;

    if std::env::var("SEED_RESET").is_ok() {
        // guard: refuse to run against anything that isn't your local dev db file
        for table in ["subclass_granted_spells", "subclass_spell_choice_grants", "subclass_spells", "class_spells", "subclass_features", "class_features", "subclasses", "classes", "spells", "backgrounds", "feats", "species", "optional_feature_prerequisite_pacts", "optional_feature_prerequisite_classes", "optional_feature_types", "optional_features", "item_properties", "items"] {
            sqlx::query(&format!("DELETE FROM {table}")).execute(&pool).await?;
        }
    }

    seed_spells(&pool, &fixtures_dir).await?;
    seed_classes(&pool, &fixtures_dir).await?;
    seed_class_spells(&pool, &fixtures_dir).await?;
    seed_backgrounds(&pool, &fixtures_dir).await?;
    seed_feats(&pool, &fixtures_dir).await?;
    seed_optional_features(&pool, &fixtures_dir).await?;
    seed_items(&pool, &fixtures_dir).await?;
    seed_species(&pool, &fixtures_dir).await?;
    Ok(())
}

async fn seed_items(pool: &SqlitePool, fixtures_dir: &str) -> anyhow::Result<()> {
    let items_raw = std::fs::read_to_string(format!("{fixtures_dir}/items.json"))?;
    let base_raw = std::fs::read_to_string(format!("{fixtures_dir}/items-base.json"))?;

    let (raw_items, raw_groups) = importer::parse_item::parse_item_file(&items_raw)?;
    let (raw_base_items, raw_properties, raw_types) =
        importer::parse_item::parse_base_item_file(&base_raw)?;

    let items = importer::transform_item::items_from_parsed(
        raw_items,
        raw_groups,
        raw_base_items,
        &raw_properties,
        &raw_types,
    )?;

    let count = items.len();
    for item in &items {
        sqlx::query(
            "INSERT INTO items (id, name, source, is_group, item_type, rarity, rarity_rank, \
             requires_attunement, weapon_category, damage_type, value_cp, weight_lb, data_json) \
             VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)",
        )
        .bind(&item.id)
        .bind(&item.name)
        .bind(&item.source)
        .bind(item.is_group)
        .bind(&item.item_type_label)
        .bind(&item.rarity)
        .bind(guidance::models::item::rarity_rank(&item.rarity))
        .bind(item.requires_attunement)
        .bind(item.weapon_category.map(|category| category.as_str()))
        .bind(item.damage.as_ref().and_then(|damage| damage.damage_type).map(|d| d.as_str()))
        .bind(item.value_cp)
        .bind(item.weight_lb)
        .bind(serde_json::to_string(item)?)
        .execute(pool)
        .await?;

        for property in &item.properties {
            sqlx::query(
                "INSERT OR IGNORE INTO item_properties (item_id, property_code, property_label) VALUES (?,?,?)",
            )
            .bind(&item.id)
            .bind(&property.code)
            .bind(&property.label)
            .execute(pool)
            .await?;
        }
    }
    println!("Seeded {count} items");
    Ok(())
}

async fn seed_species(pool: &SqlitePool, fixtures_dir: &str) -> anyhow::Result<()> {
    // 5etools exports name the species file after their "race" JSON key.
    let raw = std::fs::read_to_string(format!("{fixtures_dir}/races.json"))?;
    let species = importer::transform_species::species_from_parsed(
        importer::parse_species::parse_species_file(&raw)?,
    )?;

    let count = species.len();
    for species in species {
        sqlx::query("INSERT INTO species (id, name, source, data_json) VALUES (?,?,?,?)")
            .bind(&species.id).bind(&species.name).bind(&species.source)
            .bind(serde_json::to_string(&species)?)
            .execute(pool).await?;
    }
    println!("Seeded {count} species");
    Ok(())
}

async fn seed_backgrounds(pool: &SqlitePool, fixtures_dir: &str) -> anyhow::Result<()> {
    let raw = std::fs::read_to_string(format!("{fixtures_dir}/backgrounds.json"))?;
    let backgrounds = importer::transform_background::backgrounds_from_parsed(
        importer::parse_background::parse_background_file(&raw)?,
    )?;

    let count = backgrounds.len();
    for background in backgrounds {
        sqlx::query("INSERT INTO backgrounds (id, name, source, data_json) VALUES (?,?,?,?)")
            .bind(&background.id).bind(&background.name).bind(&background.source)
            .bind(serde_json::to_string(&background)?)
            .execute(pool).await?;
    }
    println!("Seeded {count} backgrounds");
    Ok(())
}

async fn seed_feats(pool: &SqlitePool, fixtures_dir: &str) -> anyhow::Result<()> {
    let raw = std::fs::read_to_string(format!("{fixtures_dir}/feats.json"))?;
    let feats =
        importer::transform_feat::feats_from_parsed(importer::parse_feat::parse_feat_file(&raw)?)?;

    let count = feats.len();
    for feat in feats {
        sqlx::query("INSERT INTO feats (id, name, source, data_json) VALUES (?,?,?,?)")
            .bind(&feat.id).bind(&feat.name).bind(&feat.source)
            .bind(serde_json::to_string(&feat)?)
            .execute(pool).await?;
    }
    println!("Seeded {count} feats");
    Ok(())
}

async fn seed_optional_features(pool: &SqlitePool, fixtures_dir: &str) -> anyhow::Result<()> {
    let raw = std::fs::read_to_string(format!("{fixtures_dir}/optional-features.json"))?;
    let features = importer::transform_optional_feature::optional_features_from_parsed(
        importer::parse_optional_feature::parse_optional_feature_file(&raw)?,
    )?;

    let count = features.len();
    for feature in &features {
        sqlx::query("INSERT INTO optional_features (id, name, source, data_json) VALUES (?,?,?,?)")
            .bind(&feature.id).bind(&feature.name).bind(&feature.source)
            .bind(serde_json::to_string(feature)?)
            .execute(pool).await?;

        for feature_type in &feature.feature_types {
            sqlx::query(
                "INSERT OR IGNORE INTO optional_feature_types (optional_feature_id, feature_type) VALUES (?,?)",
            )
            .bind(&feature.id)
            .bind(feature_type.as_str())
            .execute(pool)
            .await?;
        }

        for class_requirement in class_requirement_labels(&feature.prerequisites) {
            sqlx::query(
                "INSERT OR IGNORE INTO optional_feature_prerequisite_classes (optional_feature_id, class_requirement) VALUES (?,?)",
            )
            .bind(&feature.id)
            .bind(class_requirement)
            .execute(pool)
            .await?;
        }

        for pact in required_pacts(&feature.prerequisites) {
            sqlx::query(
                "INSERT OR IGNORE INTO optional_feature_prerequisite_pacts (optional_feature_id, pact) VALUES (?,?)",
            )
            .bind(&feature.id)
            .bind(pact)
            .execute(pool)
            .await?;
        }
    }
    println!("Seeded {count} optional features");
    Ok(())
}

async fn seed_spells(pool: &SqlitePool, fixtures_dir: &str) -> anyhow::Result<()> {
    let raw = std::fs::read_to_string(format!("{fixtures_dir}/spells.seed.json"))?;
    let spells = importer::transform::spells_from_parsed(importer::parse::parse_spell_array(&raw)?)?;

    let count = spells.len();
    for spell in spells {
        sqlx::query("INSERT INTO spells (id, name, level, school, source, concentration, ritual, data_json) VALUES (?,?,?,?,?,?,?,?)")
            .bind(&spell.id).bind(&spell.name).bind(spell.level).bind(spell.school.as_str())
            .bind(&spell.source)
            .bind(spell.duration.concentration).bind(spell.ritual)
            .bind(serde_json::to_string(&spell)?)
            .execute(pool).await?;
    }
    println!("Seeded {count} spells");
    Ok(())
}

async fn seed_classes(pool: &SqlitePool, fixtures_dir: &str) -> anyhow::Result<()> {
    let mut paths: Vec<_> = std::fs::read_dir(format!("{fixtures_dir}/classes"))?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    paths.sort();

    // additionalSpells grants reference spells by name only (no source), so
    // resolution is name-only too, first match wins — the same simplification
    // `seed_class_spells` makes when a spell can't be pinned to a source.
    let mut spell_ids_by_name = std::collections::HashMap::new();
    for row in sqlx::query("SELECT id, name FROM spells").fetch_all(pool).await? {
        let name: String = row.try_get("name")?;
        let id: String = row.try_get("id")?;
        spell_ids_by_name.entry(name.to_lowercase()).or_insert(id);
    }

    let mut class_count = 0;
    let mut subclass_count = 0;
    let (mut granted_linked, mut granted_skipped) = (0, 0);
    for path in paths {
        let raw = std::fs::read_to_string(&path)?;
        let bundles = importer::transform_class::class_bundles_from_parsed(
            importer::parse_class::parse_class_file(&raw)?,
        )?;

        for bundle in bundles {
            let class = &bundle.class;
            sqlx::query("INSERT INTO classes (id, name, source, hit_die, data_json) VALUES (?,?,?,?,?)")
                .bind(&class.id).bind(&class.name).bind(&class.source).bind(class.hit_die)
                .bind(serde_json::to_string(class)?)
                .execute(pool).await?;
            class_count += 1;

            for (sort_order, feature) in bundle.features.iter().enumerate() {
                sqlx::query("INSERT INTO class_features (class_id, name, source, level, sort_order, data_json) VALUES (?,?,?,?,?,?)")
                    .bind(&class.id).bind(&feature.name).bind(&feature.source).bind(feature.level)
                    .bind(sort_order as i64)
                    .bind(serde_json::to_string(feature)?)
                    .execute(pool).await?;
            }

            for (subclass, features, grants, choice_grants) in &bundle.subclasses {
                sqlx::query("INSERT INTO subclasses (id, class_id, name, short_name, source, data_json) VALUES (?,?,?,?,?,?)")
                    .bind(&subclass.id).bind(&subclass.class_id).bind(&subclass.name)
                    .bind(&subclass.short_name).bind(&subclass.source)
                    .bind(serde_json::to_string(subclass)?)
                    .execute(pool).await?;
                subclass_count += 1;

                for (sort_order, feature) in features.iter().enumerate() {
                    sqlx::query("INSERT INTO subclass_features (subclass_id, name, source, level, sort_order, data_json) VALUES (?,?,?,?,?,?)")
                        .bind(&subclass.id).bind(&feature.name).bind(&feature.source).bind(feature.level)
                        .bind(sort_order as i64)
                        .bind(serde_json::to_string(feature)?)
                        .execute(pool).await?;
                }

                for grant in grants {
                    match spell_ids_by_name.get(&grant.spell_name_lower) {
                        Some(spell_id) => {
                            sqlx::query(
                                "INSERT OR IGNORE INTO subclass_granted_spells (subclass_id, spell_id, grant_level) VALUES (?,?,?)",
                            )
                            .bind(&subclass.id)
                            .bind(spell_id)
                            .bind(grant.level)
                            .execute(pool)
                            .await?;
                            granted_linked += 1;
                        }
                        None => granted_skipped += 1,
                    }
                }

                for (grant_level, choice) in choice_grants {
                    sqlx::query(
                        "INSERT OR IGNORE INTO subclass_spell_choice_grants (subclass_id, grant_level, spell_level, class_name, school, count) VALUES (?,?,?,?,?,?)",
                    )
                    .bind(&subclass.id)
                    .bind(grant_level)
                    .bind(choice.spell_level)
                    .bind(&choice.class_name)
                    .bind(choice.school.map(|s| s.as_str()))
                    .bind(choice.count)
                    .execute(pool)
                    .await?;
                }
            }
        }
    }
    println!("Seeded {class_count} classes ({subclass_count} subclasses)");
    println!("Linked {granted_linked} subclass-granted spells ({granted_skipped} skipped, unresolved)");
    Ok(())
}

// Links classes (and subclasses, for subclass-granted casters like Eldritch
// Knight/Arcane Trickster) to castable spells from 5etools' generated
// `gendata-spell-source-lookup.json` (not present on the spell files
// themselves). Must run after both seed_spells and seed_classes, since it
// only resolves names that are already rows in those tables.
async fn seed_class_spells(pool: &SqlitePool, fixtures_dir: &str) -> anyhow::Result<()> {
    let raw = std::fs::read_to_string(format!("{fixtures_dir}/gendata-spell-source-lookup.json"))?;
    let lookup = importer::parse_class_spells::parse_class_spell_lookup(&raw)?;
    let class_links = importer::transform_class_spells::class_spell_links_from_parsed(&lookup);
    let subclass_links = importer::transform_class_spells::subclass_spell_links_from_parsed(&lookup);

    let mut spell_ids = std::collections::HashMap::new();
    for row in sqlx::query("SELECT id, name, source FROM spells").fetch_all(pool).await? {
        let name: String = row.try_get("name")?;
        let source: String = row.try_get("source")?;
        spell_ids.insert((name.to_lowercase(), source.to_uppercase()), row.try_get::<String, _>("id")?);
    }
    let mut class_ids = std::collections::HashMap::new();
    for row in sqlx::query("SELECT id, name FROM classes").fetch_all(pool).await? {
        class_ids.insert(row.try_get::<String, _>("name")?, row.try_get::<String, _>("id")?);
    }
    let mut subclass_ids = std::collections::HashMap::new();
    for row in sqlx::query(
        "SELECT s.id, s.short_name, s.source, c.name AS class_name \
         FROM subclasses s JOIN classes c ON c.id = s.class_id",
    )
    .fetch_all(pool)
    .await?
    {
        let class_name: String = row.try_get("class_name")?;
        let short_name: String = row.try_get("short_name")?;
        let source: String = row.try_get("source")?;
        subclass_ids.insert((class_name, short_name, source), row.try_get::<String, _>("id")?);
    }

    let (mut linked, mut skipped) = (0, 0);
    for link in class_links {
        let spell_id = spell_ids.get(&(link.spell_name_lower, link.spell_source.to_uppercase()));
        let class_id = class_ids.get(&link.class_name);
        match (spell_id, class_id) {
            (Some(spell_id), Some(class_id)) => {
                sqlx::query("INSERT OR IGNORE INTO class_spells (class_id, spell_id) VALUES (?, ?)")
                    .bind(class_id)
                    .bind(spell_id)
                    .execute(pool)
                    .await?;
                linked += 1;
            }
            _ => skipped += 1,
        }
    }
    println!("Linked {linked} class/spell pairs ({skipped} skipped, unresolved)");

    let (mut subclass_linked, mut subclass_skipped) = (0, 0);
    for link in subclass_links {
        let spell_id = spell_ids.get(&(link.spell_name_lower, link.spell_source.to_uppercase()));
        let subclass_id =
            subclass_ids.get(&(link.class_name, link.subclass_short_name, link.subclass_source));
        match (spell_id, subclass_id) {
            (Some(spell_id), Some(subclass_id)) => {
                sqlx::query("INSERT OR IGNORE INTO subclass_spells (subclass_id, spell_id) VALUES (?, ?)")
                    .bind(subclass_id)
                    .bind(spell_id)
                    .execute(pool)
                    .await?;
                subclass_linked += 1;
            }
            _ => subclass_skipped += 1,
        }
    }
    println!("Linked {subclass_linked} subclass/spell pairs ({subclass_skipped} skipped, unresolved)");
    Ok(())
}

// Lives in src/, not src/bin/ — Cargo auto-discovers every top-level .rs
// file directly under src/bin/ as its own binary target, which would turn
// a sibling seed_tests.rs there into a phantom standalone crate.
#[cfg(test)]
#[path = "../seed_tests.rs"]
mod tests;
