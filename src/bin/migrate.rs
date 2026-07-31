use sqlx::Row;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // `connect_dev_db()` runs any pending migrations under `./migrations` as
    // a side effect before returning — the same thing that happens on every
    // dev server start or `seed` run — so this binary's only job is to
    // trigger that on its own, for a `just migrate` step that's faster than
    // spinning up the full dev server or reseeding just to pick up new
    // migration files.
    let pool = guidance::db::connect_dev_db().await?;
    let row = sqlx::query("SELECT COUNT(*) AS count FROM _sqlx_migrations WHERE success = 1")
        .fetch_one(&pool)
        .await?;
    let count: i64 = row.try_get("count")?;
    println!("Database is up to date ({count} migrations applied).");
    Ok(())
}
