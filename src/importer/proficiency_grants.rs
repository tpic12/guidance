//! Shared walker for the raw proficiency-grant array shape 5etools uses for
//! class tools, background languages/tools, and species languages alike:
//! each element is a map of a fixed proficiency (name -> `true`), an "any"
//! wildcard, or a `choose` object with a `from` list.
use crate::models::language::LanguageGrant;
use crate::models::proficiency::{ToolCategory, ToolGrant, ToolOption};
use serde_json::Value;

/// Parses a tool-proficiency grant array (class `toolProficiencies`,
/// background `toolProficiencies`) into structured `ToolGrant`s.
pub(crate) fn tool_grants_from_raw(grants: &[Value]) -> Vec<ToolGrant> {
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
                    let from: Vec<ToolOption> = value
                        .get("from")
                        .and_then(Value::as_array)
                        .map(|list| list.iter().filter_map(Value::as_str).map(parse_tool_option).collect())
                        .unwrap_or_default();
                    out.push(ToolGrant::Choose { count, from });
                }
                "any" => out.push(ToolGrant::Any { count: value.as_u64().unwrap_or(1) as u8 }),
                "anyArtisansTool" => out.push(ToolGrant::AnyCategory {
                    count: value.as_u64().unwrap_or(1) as u8,
                    category: ToolCategory::ArtisansTool,
                }),
                "anyGamingSet" => out.push(ToolGrant::AnyCategory {
                    count: value.as_u64().unwrap_or(1) as u8,
                    category: ToolCategory::GamingSet,
                }),
                "anyMusicalInstrument" => out.push(ToolGrant::AnyCategory {
                    count: value.as_u64().unwrap_or(1) as u8,
                    category: ToolCategory::MusicalInstrument,
                }),
                name => fixed.push(name.to_string()),
            }
        }
    }
    if !fixed.is_empty() {
        out.insert(0, ToolGrant::Fixed { tools: fixed });
    }
    out
}

fn parse_tool_option(name: &str) -> ToolOption {
    match name {
        "anyArtisansTool" | "artisan's tools" => ToolOption::Category(ToolCategory::ArtisansTool),
        "anyGamingSet" | "gaming set" => ToolOption::Category(ToolCategory::GamingSet),
        "anyMusicalInstrument" | "musical instrument" => ToolOption::Category(ToolCategory::MusicalInstrument),
        other => ToolOption::Named(other.to_string()),
    }
}

/// Parses a language-proficiency grant array (background/species
/// `languageProficiencies`) into structured `LanguageGrant`s. The bare
/// `"other"` key (races.json: "one language of the DM's choice") collapses
/// into `Any{count:1}`, same bucket as `anyStandard`.
pub(crate) fn language_grants_from_raw(grants: &[Value]) -> Vec<LanguageGrant> {
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
                    out.push(LanguageGrant::Choose { count, from });
                }
                "any" | "anyStandard" => {
                    out.push(LanguageGrant::Any { count: value.as_u64().unwrap_or(1) as u8 })
                }
                "other" => out.push(LanguageGrant::Any { count: 1 }),
                name => fixed.push(name.to_string()),
            }
        }
    }
    if !fixed.is_empty() {
        out.insert(0, LanguageGrant::Fixed { languages: fixed });
    }
    out
}

#[cfg(test)]
#[path = "proficiency_grants_tests.rs"]
mod tests;
