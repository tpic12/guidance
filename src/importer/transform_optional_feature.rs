use crate::importer::entries::{clean_tags, entries_from_values};
use crate::importer::parse_optional_feature::RawOptionalFeature;
use crate::importer::transform::slugify;
use crate::models::optional_feature::{FeatureType, OptionalFeature, PrerequisiteOption, ResourceCost};
use serde_json::Value;

pub fn optional_features_from_parsed(
    raws: Vec<RawOptionalFeature>,
) -> anyhow::Result<Vec<OptionalFeature>> {
    Ok(raws.into_iter().map(optional_feature_from_raw).collect())
}

fn optional_feature_from_raw(raw: RawOptionalFeature) -> OptionalFeature {
    OptionalFeature {
        id: slugify(&raw.name),
        canonical_id: format!("{}|{}", raw.name, raw.source),
        feature_types: raw.feature_type.iter().filter_map(|code| FeatureType::from_code(code)).collect(),
        prerequisites: prerequisite_options(&raw.prerequisite),
        consumes: raw.consumes.as_ref().and_then(resource_cost_from_value),
        entries: entries_from_values(&raw.entries),
        name: raw.name,
        source: raw.source,
    }
}

/// Each element of the raw `prerequisite` array is an alternative (OR); the
/// keys within one element are requirements that must all hold (AND) — same
/// convention `transform_feat.rs`'s `prerequisite_label` already parses,
/// kept here as structured data instead of being flattened to text.
fn prerequisite_options(prerequisites: &[Value]) -> Vec<PrerequisiteOption> {
    prerequisites.iter().filter_map(|option| prerequisite_option_from_value(option.as_object()?)).collect()
}

fn prerequisite_option_from_value(option: &serde_json::Map<String, Value>) -> Option<PrerequisiteOption> {
    let mut result = PrerequisiteOption::default();

    if let Some(level) = option.get("level") {
        result.level = level.get("level").and_then(Value::as_u64).map(|l| l as u8);
        if let Some(class) = level.get("class") {
            result.class_name = class.get("name").and_then(Value::as_str).map(str::to_string);
            result.class_source = class.get("source").and_then(Value::as_str).map(str::to_string);
        }
        if let Some(subclass) = level.get("subclass") {
            result.subclass_name = subclass.get("name").and_then(Value::as_str).map(str::to_string);
            result.subclass_source = subclass.get("source").and_then(Value::as_str).map(str::to_string);
        }
    }
    if let Some(pact) = option.get("pact").and_then(Value::as_str) {
        result.pact = Some(pact.to_string());
    }
    if let Some(spells) = option.get("spell").and_then(Value::as_array) {
        result.spells = spells
            .iter()
            .filter_map(Value::as_str)
            .filter_map(|name| {
                let clean = name.split(['#', '|']).next().unwrap_or(name).trim();
                (!clean.is_empty()).then(|| clean.to_lowercase())
            })
            .collect();
    }
    if let Some(items) = option.get("item").and_then(Value::as_array) {
        let names: Vec<String> = items.iter().filter_map(Value::as_str).map(clean_tags).collect();
        if !names.is_empty() {
            result.other = Some(names.join(" or "));
        }
    }

    (result != PrerequisiteOption::default()).then_some(result)
}

fn resource_cost_from_value(value: &Value) -> Option<ResourceCost> {
    let name = value.get("name").and_then(Value::as_str)?.to_string();
    Some(ResourceCost {
        name,
        amount: value.get("amount").and_then(Value::as_u64).map(|n| n as u32),
        amount_min: value.get("amountMin").and_then(Value::as_u64).map(|n| n as u32),
        amount_max: value.get("amountMax").and_then(Value::as_u64).map(|n| n as u32),
    })
}

#[cfg(test)]
#[path = "transform_optional_feature_tests.rs"]
mod tests;
