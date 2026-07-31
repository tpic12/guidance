use anyhow::Context;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct RawSpecies {
    pub name: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub size: Vec<String>,
    #[serde(default)]
    pub speed: Value,
    #[serde(default)]
    pub ability: Vec<Value>,
    #[serde(default)]
    pub darkvision: Option<u32>,
    #[serde(default)]
    pub entries: Vec<Value>,
    /// Stub records that inherit another record's content; skipped for now.
    #[serde(default, rename = "_copy")]
    pub copy: Option<Value>,
}

pub fn parse_species_file(raw: &str) -> anyhow::Result<Vec<RawSpecies>> {
    let root: Value = serde_json::from_str(raw)?;
    // 5etools exports still key species under "race".
    root.get("race")
        .and_then(Value::as_array)
        .context("missing top-level race array")?
        .iter()
        .map(|record| {
            let name = record
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("<unnamed>");
            serde_json::from_value(record.clone())
                .with_context(|| format!("failed to parse species '{name}'"))
        })
        .collect()
}
