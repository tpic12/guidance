use crate::models::spell::{Spell, SpellQuery, SpellSort};
use anyhow::Context;
use sqlx::sqlite::SqlitePool;
use sqlx::{QueryBuilder, Row, Sqlite};

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
