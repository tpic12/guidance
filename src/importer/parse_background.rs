use anyhow::{bail, Context};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawBackground {
    pub name: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub skill_proficiencies: Vec<Value>,
    #[serde(default)]
    pub language_proficiencies: Vec<Value>,
    #[serde(default)]
    pub tool_proficiencies: Vec<Value>,
    #[serde(default)]
    pub entries: Vec<Value>,
}

pub fn parse_background_file(raw: &str) -> anyhow::Result<Vec<RawBackground>> {
    let root: Value = serde_json::from_str(raw)?;
    let records = root
        .get("background")
        .and_then(Value::as_array)
        .context("missing top-level background array")?;

    resolve_copies(records)?
        .iter()
        .map(|record| {
            serde_json::from_value(record.clone())
                .with_context(|| format!("failed to parse background '{}'", record_name(record)))
        })
        .collect()
}

/// Variant records in the source data use `_copy` to inherit another record's
/// fields and optionally patch its `entries` via `_mod`, so the transform only
/// ever sees complete records after this. Copies can chain (a copy of a copy),
/// hence the fixpoint loop.
fn resolve_copies(records: &[Value]) -> anyhow::Result<Vec<Value>> {
    let mut resolved: Vec<Value> = records.to_vec();

    loop {
        let complete: HashMap<String, Value> = resolved
            .iter()
            .filter(|record| record.get("_copy").is_none())
            .map(|record| (record_name(record).to_lowercase(), record.clone()))
            .collect();

        let mut progressed = false;
        for record in &mut resolved {
            let Some(copy) = record.get("_copy") else {
                continue;
            };
            let target = copy.get("name").and_then(Value::as_str).unwrap_or_default();
            if let Some(base) = complete.get(&target.to_lowercase()) {
                *record = apply_copy(base.clone(), record)?;
                progressed = true;
            }
        }

        if resolved.iter().all(|record| record.get("_copy").is_none()) {
            return Ok(resolved);
        }
        if !progressed {
            let stuck: Vec<&str> = resolved
                .iter()
                .filter(|r| r.get("_copy").is_some())
                .map(record_name)
                .collect();
            bail!(
                "could not resolve _copy for backgrounds: {}",
                stuck.join(", ")
            );
        }
    }
}

/// Builds the resolved record: the base's fields, overlaid with everything the
/// copy declares itself, then the copy's `_mod` array patches applied on top.
fn apply_copy(base: Value, copy: &Value) -> anyhow::Result<Value> {
    let name = record_name(copy).to_string();
    let mut merged = base;
    let (Some(merged_obj), Some(copy_obj)) = (merged.as_object_mut(), copy.as_object()) else {
        bail!("background record '{name}' is not a JSON object");
    };

    for (key, value) in copy_obj {
        if key != "_copy" {
            merged_obj.insert(key.clone(), value.clone());
        }
    }

    if let Some(mods) = copy_obj
        .get("_copy")
        .and_then(|c| c.get("_mod"))
        .and_then(Value::as_object)
    {
        for (prop, prop_mods) in mods {
            let mut target = merged_obj
                .get(prop)
                .and_then(Value::as_array)
                .cloned()
                .with_context(|| format!("_mod targets missing array '{prop}' on '{name}'"))?;
            // A prop's mods are a single object or an array of them.
            let prop_mods = match prop_mods {
                Value::Array(list) => list.clone(),
                single => vec![single.clone()],
            };
            for prop_mod in &prop_mods {
                apply_array_mod(&mut target, prop_mod)
                    .with_context(|| format!("applying _mod on '{prop}' of '{name}'"))?;
            }
            merged_obj.insert(prop.clone(), Value::Array(target));
        }
    }

    Ok(merged)
}

fn apply_array_mod(target: &mut Vec<Value>, prop_mod: &Value) -> anyhow::Result<()> {
    let items = || -> Vec<Value> {
        match prop_mod.get("items") {
            Some(Value::Array(items)) => items.clone(),
            Some(single) => vec![single.clone()],
            None => Vec::new(),
        }
    };

    match prop_mod
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or_default()
    {
        "appendArr" => target.extend(items()),
        "insertArr" => {
            let index = prop_mod.get("index").and_then(Value::as_u64).unwrap_or(0) as usize;
            let index = index.min(target.len());
            target.splice(index..index, items());
        }
        "replaceArr" => {
            // `replace` names the entry to swap out, or addresses it by index.
            let replace = prop_mod.get("replace").unwrap_or(&Value::Null);
            let index = match replace {
                Value::String(name) => target.iter().position(|entry| {
                    entry.get("name").and_then(Value::as_str) == Some(name)
                        || entry.as_str() == Some(name)
                }),
                other => other
                    .get("index")
                    .and_then(Value::as_u64)
                    .map(|index| index as usize),
            }
            .filter(|index| *index < target.len())
            .with_context(|| format!("replaceArr target {replace} not found"))?;
            target.splice(index..=index, items());
        }
        other => bail!("unsupported _mod mode '{other}'"),
    }
    Ok(())
}

fn record_name(record: &Value) -> &str {
    record
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("<unnamed>")
}

#[cfg(test)]
#[path = "parse_background_tests.rs"]
mod tests;
