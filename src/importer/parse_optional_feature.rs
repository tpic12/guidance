use anyhow::Context;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct RawOptionalFeature {
    pub name: String,
    #[serde(default)]
    pub source: String,
    #[serde(default, rename = "featureType")]
    pub feature_type: Vec<String>,
    #[serde(default)]
    pub prerequisite: Vec<Value>,
    pub consumes: Option<Value>,
    #[serde(default)]
    pub entries: Vec<Value>,
}

pub fn parse_optional_feature_file(raw: &str) -> anyhow::Result<Vec<RawOptionalFeature>> {
    let root: Value = serde_json::from_str(raw)?;
    root.get("optionalfeature")
        .and_then(Value::as_array)
        .context("missing top-level optionalfeature array")?
        .iter()
        .map(|record| {
            let name = record.get("name").and_then(Value::as_str).unwrap_or("<unnamed>");
            serde_json::from_value(record.clone())
                .with_context(|| format!("failed to parse optional feature '{name}'"))
        })
        .collect()
}
