use anyhow::Context;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct RawLanguage {
    pub name: String,
    #[serde(default)]
    pub source: String,
    #[serde(default, rename = "type")]
    pub language_type: Option<String>,
    #[serde(default)]
    pub script: Option<String>,
}

pub fn parse_language_file(raw: &str) -> anyhow::Result<Vec<RawLanguage>> {
    let root: Value = serde_json::from_str(raw)?;
    root.get("language")
        .and_then(Value::as_array)
        .context("missing top-level language array")?
        .iter()
        .map(|record| {
            let name = record
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("<unnamed>");
            serde_json::from_value(record.clone())
                .with_context(|| format!("failed to parse language '{name}'"))
        })
        .collect()
}
