use anyhow::Context;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct RawFeat {
    pub name: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub prerequisite: Vec<Value>,
    #[serde(default)]
    pub ability: Vec<Value>,
    #[serde(default)]
    pub entries: Vec<Value>,
}

pub fn parse_feat_file(raw: &str) -> anyhow::Result<Vec<RawFeat>> {
    let root: Value = serde_json::from_str(raw)?;
    root.get("feat")
        .and_then(Value::as_array)
        .context("missing top-level feat array")?
        .iter()
        .map(|record| {
            let name = record
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("<unnamed>");
            serde_json::from_value(record.clone())
                .with_context(|| format!("failed to parse feat '{name}'"))
        })
        .collect()
}
