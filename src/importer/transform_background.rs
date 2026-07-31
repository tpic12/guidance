use crate::importer::entries::entries_from_values;
use crate::importer::parse_background::RawBackground;
use crate::importer::transform::slugify;
use crate::models::background::Background;
use crate::models::skill::SkillGrant;
use serde_json::Value;

pub fn backgrounds_from_parsed(raws: Vec<RawBackground>) -> anyhow::Result<Vec<Background>> {
    Ok(raws.into_iter().map(background_from_raw).collect())
}

fn background_from_raw(raw: RawBackground) -> Background {
    Background {
        id: slugify(&raw.name),
        canonical_id: format!("{}|{}", raw.name, raw.source),
        skills: skill_grants_from_background(&raw.skill_proficiencies),
        languages: proficiency_labels(&raw.language_proficiencies),
        tools: proficiency_labels(&raw.tool_proficiencies),
        entries: entries_from_values(&raw.entries),
        name: raw.name,
        source: raw.source,
    }
}

/// Parses a skill-proficiency grant array into structured `SkillGrant`s,
/// preserving the count/options data `proficiency_labels` flattens away.
/// Each element is a map of grants: a concrete skill keyed by its lowercase
/// name (value `true`), an "any"-style wildcard, or a `choose` object with a
/// `from` list — the same raw shape `proficiency_labels` walks below.
pub(crate) fn skill_grants_from_background(grants: &[Value]) -> Vec<SkillGrant> {
    let mut fixed = Vec::new();
    let mut out = Vec::new();
    for grant in grants {
        let Some(grant) = grant.as_object() else {
            continue;
        };
        for (key, value) in grant {
            match key.as_str() {
                "choose" => {
                    let count = value.get("count").and_then(Value::as_u64).unwrap_or(1) as u8;
                    let from: Vec<String> = value
                        .get("from")
                        .and_then(Value::as_array)
                        .map(|list| list.iter().filter_map(Value::as_str).map(str::to_string).collect())
                        .unwrap_or_default();
                    out.push(SkillGrant::Choose { count, from });
                }
                "any" | "anyStandard" => {
                    out.push(SkillGrant::Any { count: value.as_u64().unwrap_or(1) as u8 });
                }
                name => fixed.push(name.to_string()),
            }
        }
    }
    if !fixed.is_empty() {
        out.insert(0, SkillGrant::Fixed { skills: fixed });
    }
    out
}

/// Turns one proficiency-grant array into display labels. Each element is a
/// map of grants: a concrete proficiency keyed by its lowercase name (value
/// `true`), an "any"-style wildcard keyed by category (value = how many), or a
/// `choose` object with a `from` list.
pub(crate) fn proficiency_labels(grants: &[Value]) -> Vec<String> {
    let mut labels = Vec::new();
    for grant in grants {
        let Some(grant) = grant.as_object() else {
            continue;
        };
        for (key, value) in grant {
            labels.push(match key.as_str() {
                "choose" => {
                    let count = value.get("count").and_then(Value::as_u64).unwrap_or(1);
                    let options = value
                        .get("from")
                        .and_then(Value::as_array)
                        .map(|from| {
                            from.iter()
                                .filter_map(Value::as_str)
                                .map(title_case)
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .unwrap_or_default();
                    format!("Choose {count} from {options}")
                }
                "any" | "anyStandard" => {
                    format!("Any {} of your choice", value.as_u64().unwrap_or(1))
                }
                "anyArtisansTool" => any_category_label("artisan's tools", value),
                "anyGamingSet" => any_category_label("gaming set", value),
                "anyMusicalInstrument" => any_category_label("musical instrument", value),
                name => title_case(name),
            });
        }
    }
    labels
}

fn any_category_label(category: &str, count: &Value) -> String {
    match count.as_u64().unwrap_or(1) {
        1 => format!("Any {category}"),
        n => format!("Any {n} {category}"),
    }
}

/// "sleight of hand" -> "Sleight of Hand" (connecting words stay lowercase).
pub(crate) fn title_case(name: &str) -> String {
    name.split(' ')
        .enumerate()
        .map(|(i, word)| match word {
            "of" | "the" | "and" | "or" if i > 0 => word.to_string(),
            _ => {
                let mut chars = word.chars();
                match chars.next() {
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                    None => String::new(),
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
#[path = "transform_background_tests.rs"]
mod tests;
