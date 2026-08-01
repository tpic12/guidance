use crate::models::background::{Background, BackgroundQuery, BackgroundSort};
use crate::models::character::{active_subclass, total_level, AsiChoice, Character, CharacterSheet, CharacterSummary};
use crate::models::class::{Class, ClassDetail, ClassFeature, ClassSummary, Subclass, SubclassDetail};
use crate::models::feat::{Feat, FeatQuery, FeatSort};
use crate::models::item::{Item, ItemPropertyOption, ItemQuery, ItemSort};
use crate::models::optional_feature::{OptionalFeature, OptionalFeatureQuery, OptionalFeatureSort};
use crate::models::species::{Species, SpeciesQuery, SpeciesSort};
use crate::models::spell::{Spell, SpellQuery, SpellSort};
use crate::models::user::{CurrentUser, Permission, UserSummary};
use anyhow::{bail, Context};
use argon2::password_hash::{rand_core::OsRng, SaltString};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use sqlx::migrate::MigrateDatabase;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions};
use sqlx::{QueryBuilder, Row, Sqlite};
use std::str::FromStr;

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

/// Lists spells matching the given search/filter/sort parameters. Reads the
/// full `Spell` back out of each row's `data_json` column, so callers get
/// complete records (usable for both the compendium table and detail view)
/// without a second query.
pub async fn list_spells(pool: &SqlitePool, filter: &SpellQuery) -> anyhow::Result<Vec<Spell>> {
    let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new("SELECT data_json FROM spells");
    let mut where_started = false;

    if !filter.schools.is_empty() {
        qb.push(" WHERE school IN (");
        where_started = true;
        let mut separated = qb.separated(", ");
        for school in &filter.schools {
            separated.push_bind(school.as_str());
        }
        separated.push_unseparated(")");
    }

    if !filter.levels.is_empty() {
        qb.push(if where_started { " AND " } else { " WHERE " });
        where_started = true;
        qb.push("level IN (");
        let mut separated = qb.separated(", ");
        for level in &filter.levels {
            separated.push_bind(*level as i64);
        }
        separated.push_unseparated(")");
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

    if !filter.classes.is_empty() {
        qb.push(if where_started { " AND " } else { " WHERE " });
        where_started = true;
        qb.push(
            "id IN (SELECT cs.spell_id FROM class_spells cs JOIN classes c ON c.id = cs.class_id WHERE c.name IN (",
        );
        let mut separated = qb.separated(", ");
        for class_name in &filter.classes {
            separated.push_bind(class_name);
        }
        separated.push_unseparated("))");
    }

    let search = filter.search.trim();
    if !search.is_empty() {
        qb.push(if where_started { " AND " } else { " WHERE " });
        qb.push("name LIKE ").push_bind(format!("%{search}%"));
    }

    qb.push(match filter.sort {
        SpellSort::NameAsc => " ORDER BY name ASC",
        SpellSort::NameDesc => " ORDER BY name DESC",
        SpellSort::LevelAsc => " ORDER BY level ASC, name ASC",
        SpellSort::LevelDesc => " ORDER BY level DESC, name ASC",
        SpellSort::SchoolAsc => " ORDER BY school ASC, name ASC",
        SpellSort::SchoolDesc => " ORDER BY school DESC, name ASC",
    });

    let rows = qb.build().fetch_all(pool).await.context("failed to query spells")?;

    rows.into_iter()
        .map(|row| {
            let data_json: String =
                row.try_get("data_json").context("missing data_json column")?;
            serde_json::from_str::<Spell>(&data_json).context("failed to deserialize spell row")
        })
        .collect()
}

/// Lists spells castable by a given class, up to (and including) the given
/// spell level — backs the character builder's Spells step and its
/// server-side re-validation in `save_character`. `subclass_id` additionally
/// pulls in spells linked via `subclass_spells`, for subclass-granted casters
/// (Eldritch Knight, Arcane Trickster, ...) whose base class has no spells of
/// its own linked at all; `UNION` (not `UNION ALL`) dedupes the rare spell
/// reachable via both the class and its subclass. Spells the subclass
/// auto-grants at or below `level` (see `subclass_granted_spells`/
/// `get_subclass_granted_spells`) are excluded from this pickable pool —
/// they're never a choice, just always there.
pub async fn list_spells_for_class(
    pool: &SqlitePool,
    class_id: &str,
    subclass_id: Option<&str>,
    level: u8,
    max_spell_level: u8,
) -> anyhow::Result<Vec<Spell>> {
    let base_query = "SELECT s.id, s.data_json, s.level, s.name FROM spells s \
         JOIN class_spells cs ON cs.spell_id = s.id \
         WHERE cs.class_id = ? AND s.level <= ?";
    let subclass_query = "SELECT s.id, s.data_json, s.level, s.name FROM spells s \
         JOIN subclass_spells scs ON scs.spell_id = s.id \
         WHERE scs.subclass_id = ? AND s.level <= ?";

    let sql = match subclass_id {
        Some(_) => format!(
            "SELECT * FROM ({base_query} UNION {subclass_query}) \
             WHERE id NOT IN (SELECT spell_id FROM subclass_granted_spells WHERE subclass_id = ? AND grant_level <= ?) \
             ORDER BY level, name"
        ),
        None => format!("{base_query} ORDER BY level, name"),
    };
    let mut query = sqlx::query(&sql).bind(class_id).bind(max_spell_level as i64);
    if let Some(subclass_id) = subclass_id {
        query = query.bind(subclass_id).bind(max_spell_level as i64).bind(subclass_id).bind(level as i64);
    }

    let rows = query.fetch_all(pool).await.context("failed to query spells for class")?;

    rows.into_iter()
        .map(|row| {
            let data_json: String = row.try_get("data_json").context("missing data_json column")?;
            serde_json::from_str::<Spell>(&data_json).context("failed to deserialize spell row")
        })
        .collect()
}

/// Spells a subclass auto-grants at or below `level` (Cleric domain spells,
/// Druid circle spells, ...) — always active, never a choice, and excluded
/// from `list_spells_for_class`'s pickable pool. Ordered by grant level then
/// name so callers can render "granted at level N" groupings if useful.
pub async fn get_subclass_granted_spells(
    pool: &SqlitePool,
    subclass_id: &str,
    level: u8,
) -> anyhow::Result<Vec<Spell>> {
    let rows = sqlx::query(
        "SELECT s.data_json FROM spells s \
         JOIN subclass_granted_spells gs ON gs.spell_id = s.id \
         WHERE gs.subclass_id = ? AND gs.grant_level <= ? \
         ORDER BY gs.grant_level, s.name",
    )
    .bind(subclass_id)
    .bind(level as i64)
    .fetch_all(pool)
    .await
    .context("failed to query subclass-granted spells")?;

    rows.into_iter()
        .map(|row| {
            let data_json: String = row.try_get("data_json").context("missing data_json column")?;
            serde_json::from_str::<Spell>(&data_json).context("failed to deserialize spell row")
        })
        .collect()
}

/// Spell choice-pools a subclass's `{"choose": ...}` grants unlock at or
/// below `level`, one `(count, Vec<Spell>)` per filter row (not merged).
pub async fn get_subclass_spell_choice_pools(
    pool: &SqlitePool,
    subclass_id: &str,
    level: u8,
) -> anyhow::Result<Vec<(u8, Vec<Spell>)>> {
    let rows = sqlx::query(
        "SELECT spell_level, class_name, school, count FROM subclass_spell_choice_grants \
         WHERE subclass_id = ? AND grant_level <= ? ORDER BY id",
    )
    .bind(subclass_id)
    .bind(level as i64)
    .fetch_all(pool)
    .await
    .context("failed to query subclass spell choice grants")?;

    let mut pools = Vec::with_capacity(rows.len());
    for row in rows {
        let spell_level: i64 = row.try_get("spell_level").context("missing spell_level column")?;
        let class_name: Option<String> = row.try_get("class_name").context("missing class_name column")?;
        let school: Option<String> = row.try_get("school").context("missing school column")?;
        let count: i64 = row.try_get("count").context("missing count column")?;

        let sql = match (&class_name, &school) {
            (Some(_), Some(_)) => {
                "SELECT s.data_json FROM spells s \
                 JOIN class_spells cs ON cs.spell_id = s.id \
                 JOIN classes c ON c.id = cs.class_id \
                 WHERE s.level = ? AND c.name = ? AND s.school = ? ORDER BY s.name"
            }
            (Some(_), None) => {
                "SELECT s.data_json FROM spells s \
                 JOIN class_spells cs ON cs.spell_id = s.id \
                 JOIN classes c ON c.id = cs.class_id \
                 WHERE s.level = ? AND c.name = ? ORDER BY s.name"
            }
            (None, Some(_)) => "SELECT s.data_json FROM spells s WHERE s.level = ? AND s.school = ? ORDER BY s.name",
            (None, None) => "SELECT s.data_json FROM spells s WHERE s.level = ? ORDER BY s.name",
        };
        let mut query = sqlx::query(sql).bind(spell_level);
        if let Some(class_name) = &class_name {
            query = query.bind(class_name);
        }
        if let Some(school) = &school {
            query = query.bind(school);
        }
        let spell_rows = query.fetch_all(pool).await.context("failed to query subclass spell choice pool")?;
        let spells = spell_rows
            .into_iter()
            .map(|row| {
                let data_json: String = row.try_get("data_json").context("missing data_json column")?;
                serde_json::from_str::<Spell>(&data_json).context("failed to deserialize spell row")
            })
            .collect::<anyhow::Result<Vec<Spell>>>()?;
        pools.push((count as u8, spells));
    }
    Ok(pools)
}

/// Resolves spell ids back into full `Spell` records, tolerating ids that no
/// longer exist (e.g. a spell removed after a character already picked it) —
/// same tolerate-dangling policy as the rest of character sheet resolution.
pub async fn get_spells_by_ids(pool: &SqlitePool, ids: &[String]) -> anyhow::Result<Vec<Spell>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new("SELECT data_json FROM spells WHERE id IN (");
    let mut separated = qb.separated(", ");
    for id in ids {
        separated.push_bind(id);
    }
    separated.push_unseparated(")");

    let rows = qb.build().fetch_all(pool).await.context("failed to query spells by id")?;

    rows.into_iter()
        .map(|row| {
            let data_json: String = row.try_get("data_json").context("missing data_json column")?;
            serde_json::from_str::<Spell>(&data_json).context("failed to deserialize spell row")
        })
        .collect()
}

/// Lists the distinct source books currently present in the spells table, for
/// populating the source filter's option list.
pub async fn list_spell_sources(pool: &SqlitePool) -> anyhow::Result<Vec<String>> {
    let rows = sqlx::query("SELECT DISTINCT source FROM spells ORDER BY source")
        .fetch_all(pool)
        .await
        .context("failed to query spell sources")?;

    rows.into_iter()
        .map(|row| row.try_get::<String, _>("source").context("missing source column"))
        .collect()
}

/// Lists the distinct classes with at least one spell linked via
/// `class_spells`, for populating the class filter's option list.
pub async fn list_spell_classes(pool: &SqlitePool) -> anyhow::Result<Vec<String>> {
    let rows = sqlx::query(
        "SELECT DISTINCT c.name FROM classes c JOIN class_spells cs ON cs.class_id = c.id ORDER BY c.name",
    )
    .fetch_all(pool)
    .await
    .context("failed to query spell classes")?;

    rows.into_iter().map(|row| row.try_get::<String, _>("name").context("missing name column")).collect()
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

/// So every fresh Docker Compose deploy has a working login with no manual
/// setup step. Safe to call on every boot: a no-op once any user exists.
pub async fn bootstrap_admin_if_empty(pool: &SqlitePool) -> anyhow::Result<()> {
    let row = sqlx::query("SELECT COUNT(*) AS count FROM users")
        .fetch_one(pool)
        .await
        .context("failed to count users")?;
    let count: i64 = row.try_get("count").context("missing count column")?;
    if count > 0 {
        return Ok(());
    }
    create_user(pool, "admin", "admin", true)
        .await
        .context("failed to bootstrap admin user")?;
    Ok(())
}

fn hash_password(password: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|err| anyhow::anyhow!("failed to hash password: {err}"))
}

fn verify_password(password: &str, hash: &str) -> anyhow::Result<bool> {
    let parsed =
        PasswordHash::new(hash).map_err(|err| anyhow::anyhow!("corrupt password hash: {err}"))?;
    Ok(Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok())
}

fn permission_from_str(value: &str) -> anyhow::Result<Permission> {
    Permission::ALL
        .into_iter()
        .find(|permission| permission.as_str() == value)
        .ok_or_else(|| anyhow::anyhow!("unknown permission: {value}"))
}

fn map_unique_violation(err: sqlx::Error, message: &'static str) -> anyhow::Error {
    match &err {
        sqlx::Error::Database(db_err) if db_err.is_unique_violation() => anyhow::anyhow!(message),
        _ => anyhow::Error::from(err).context(message),
    }
}

/// `is_instance_owner` must only ever be `true` for the bootstrap admin
/// account created by `bootstrap_admin_if_empty` — there is deliberately no
/// function anywhere in this module to flip it after creation, since its
/// permanence is what guarantees the instance can never be locked out of
/// admin access.
pub async fn create_user(
    pool: &SqlitePool,
    username: &str,
    password: &str,
    is_instance_owner: bool,
) -> anyhow::Result<String> {
    let id = uuid::Uuid::new_v4().to_string();
    let password_hash = hash_password(password)?;
    sqlx::query(
        "INSERT INTO users (id, username, password_hash, is_instance_owner, theme) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(username)
    .bind(&password_hash)
    .bind(is_instance_owner)
    .bind("guidance")
    .execute(pool)
    .await
    .map_err(|err| map_unique_violation(err, "username already taken"))?;
    Ok(id)
}

pub async fn authenticate(
    pool: &SqlitePool,
    username: &str,
    password: &str,
) -> anyhow::Result<Option<String>> {
    let Some(row) = sqlx::query("SELECT id, password_hash FROM users WHERE username = ?")
        .bind(username)
        .fetch_optional(pool)
        .await
        .context("failed to query user by username")?
    else {
        return Ok(None);
    };

    let id: String = row.try_get("id").context("missing id column")?;
    let password_hash: String =
        row.try_get("password_hash").context("missing password_hash column")?;

    if verify_password(password, &password_hash)? {
        Ok(Some(id))
    } else {
        Ok(None)
    }
}

async fn list_permissions_for_user(
    pool: &SqlitePool,
    user_id: &str,
) -> anyhow::Result<Vec<Permission>> {
    sqlx::query("SELECT permission FROM user_permissions WHERE user_id = ?")
        .bind(user_id)
        .fetch_all(pool)
        .await
        .context("failed to query user permissions")?
        .into_iter()
        .map(|row| {
            let permission: String =
                row.try_get("permission").context("missing permission column")?;
            permission_from_str(&permission)
        })
        .collect()
}

pub async fn get_current_user(
    pool: &SqlitePool,
    user_id: &str,
) -> anyhow::Result<Option<CurrentUser>> {
    let Some(row) =
        sqlx::query("SELECT id, username, is_instance_owner, theme FROM users WHERE id = ?")
            .bind(user_id)
            .fetch_optional(pool)
            .await
            .context("failed to query user")?
    else {
        return Ok(None);
    };

    let id: String = row.try_get("id").context("missing id column")?;
    let permissions = list_permissions_for_user(pool, &id).await?;

    Ok(Some(CurrentUser {
        id,
        username: row.try_get("username").context("missing username column")?,
        is_instance_owner: row
            .try_get("is_instance_owner")
            .context("missing is_instance_owner column")?,
        permissions,
        theme: row.try_get("theme").context("missing theme column")?,
    }))
}

pub async fn list_users(pool: &SqlitePool) -> anyhow::Result<Vec<UserSummary>> {
    let rows = sqlx::query("SELECT id, username, is_instance_owner FROM users ORDER BY username")
        .fetch_all(pool)
        .await
        .context("failed to query users")?;

    let mut users = Vec::with_capacity(rows.len());
    for row in rows {
        let id: String = row.try_get("id").context("missing id column")?;
        let username: String = row.try_get("username").context("missing username column")?;
        let is_instance_owner: bool = row
            .try_get("is_instance_owner")
            .context("missing is_instance_owner column")?;
        let permissions = list_permissions_for_user(pool, &id).await?;
        users.push(UserSummary { id, username, is_instance_owner, permissions });
    }
    Ok(users)
}

pub async fn update_username(
    pool: &SqlitePool,
    user_id: &str,
    new_username: &str,
) -> anyhow::Result<()> {
    sqlx::query("UPDATE users SET username = ? WHERE id = ?")
        .bind(new_username)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|err| map_unique_violation(err, "username already taken"))?;
    Ok(())
}

pub async fn update_password(
    pool: &SqlitePool,
    user_id: &str,
    new_password: &str,
) -> anyhow::Result<()> {
    let password_hash = hash_password(new_password)?;
    sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
        .bind(password_hash)
        .bind(user_id)
        .execute(pool)
        .await
        .context("failed to update password")?;
    Ok(())
}

/// Caller must validate `theme` against `models::user::THEMES` first — this
/// just persists whatever string it's given.
pub async fn update_theme(pool: &SqlitePool, user_id: &str, theme: &str) -> anyhow::Result<()> {
    sqlx::query("UPDATE users SET theme = ? WHERE id = ?")
        .bind(theme)
        .bind(user_id)
        .execute(pool)
        .await
        .context("failed to update theme")?;
    Ok(())
}

/// Re-granting a permission a user already has is a no-op, not an error.
pub async fn grant_permission(
    pool: &SqlitePool,
    user_id: &str,
    permission: Permission,
    granted_by: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT OR IGNORE INTO user_permissions (user_id, permission, granted_by) VALUES (?, ?, ?)",
    )
    .bind(user_id)
    .bind(permission.as_str())
    .bind(granted_by)
    .execute(pool)
    .await
    .context("failed to grant permission")?;
    Ok(())
}

pub async fn revoke_permission(
    pool: &SqlitePool,
    user_id: &str,
    permission: Permission,
) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM user_permissions WHERE user_id = ? AND permission = ?")
        .bind(user_id)
        .bind(permission.as_str())
        .execute(pool)
        .await
        .context("failed to revoke permission")?;
    Ok(())
}

#[cfg(test)]
#[path = "db_tests.rs"]
mod tests;
