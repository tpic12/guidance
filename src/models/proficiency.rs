use crate::models::language::LanguageGrant;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The 5etools item-type codes that group tool items into a choosable
/// category (`anyArtisansTool`/`anyGamingSet`/`anyMusicalInstrument` in
/// source data) — resolved against the `items` table's `item_type_code`
/// column at read time rather than a hardcoded member list.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ToolCategory {
    ArtisansTool,
    GamingSet,
    MusicalInstrument,
}

impl ToolCategory {
    pub fn item_type_code(&self) -> &'static str {
        match self {
            ToolCategory::ArtisansTool => "AT",
            ToolCategory::GamingSet => "GS",
            ToolCategory::MusicalInstrument => "INS",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            ToolCategory::ArtisansTool => "artisan's tools",
            ToolCategory::GamingSet => "gaming set",
            ToolCategory::MusicalInstrument => "musical instrument",
        }
    }
}

/// One entry of a `Choose` tool grant's `from` list: raw 5etools data mixes
/// concrete tool names and category tokens in the same array (e.g. Far
/// Traveler: `{"choose":{"from":["musical instrument","gaming set"]}}`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", content = "value")]
pub enum ToolOption {
    Named(String),
    Category(ToolCategory),
}

/// Mirrors `SkillGrant`, plus `AnyCategory` for tool-category-scoped choices
/// that skills/languages have no equivalent of.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind")]
pub enum ToolGrant {
    /// Tools granted outright, no player choice.
    Fixed { tools: Vec<String> },
    /// Choose `count` from a specific list of candidate tools/categories.
    Choose { count: u8, from: Vec<ToolOption> },
    /// Choose `count` from all tools ("any": N in source data).
    Any { count: u8 },
    /// Choose `count` from one tool category (e.g. "anyArtisansTool": 1).
    AnyCategory { count: u8, category: ToolCategory },
}

/// Renders grants back into "Choose 1 from Thieves' Tools, ..." / "Any
/// artisan's tools" text — derived from the structured grants so display can
/// never drift out of sync with them.
pub fn describe_tool_grants(grants: &[ToolGrant]) -> String {
    grants
        .iter()
        .map(|grant| match grant {
            ToolGrant::Fixed { tools } => {
                tools.iter().map(|t| title_case(t)).collect::<Vec<_>>().join(", ")
            }
            ToolGrant::Choose { count, from } => {
                let options = from.iter().map(tool_option_label).collect::<Vec<_>>().join(", ");
                format!("Choose {count} from {options}")
            }
            ToolGrant::Any { count } => format!("Any {count}"),
            ToolGrant::AnyCategory { count, category } => match count {
                1 => format!("Any {}", category.label()),
                n => format!("Any {n} {}", category.label()),
            },
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn tool_option_label(option: &ToolOption) -> String {
    match option {
        ToolOption::Named(name) => title_case(name),
        ToolOption::Category(category) => title_case(category.label()),
    }
}

/// True if `needle` names the same proficiency as some entry in `haystack`,
/// ignoring case. Fixed grants store 5etools' raw lowercase import keys
/// ("common", "alchemist's supplies"); DB-backed pool options (`Language`/
/// `Item` names, e.g. "Common", "Alchemist's supplies") don't reliably
/// title-case back to those keys (many item names only capitalize their
/// first word), so membership checks between the two must compare
/// case-insensitively rather than assume a shared casing convention.
pub fn contains_ignore_case(haystack: &[String], needle: &str) -> bool {
    haystack.iter().any(|item| item.eq_ignore_ascii_case(needle))
}

/// Merges a fixed-grant list with the player's chosen picks into one
/// deduped, display-ready list — case-insensitively, for the same reason as
/// `contains_ignore_case`. When a name appears on both sides (e.g. fixed-
/// granted "common" and separately chosen "Common" via an `Any` slot),
/// keeps the choice's DB-sourced casing over the fixed grant's raw one.
pub fn merge_resolved_names(fixed: &[String], choices: &[String]) -> Vec<String> {
    let mut by_key: HashMap<String, String> = HashMap::new();
    for name in fixed {
        by_key.insert(name.to_lowercase(), title_case(name));
    }
    for name in choices {
        by_key.insert(name.to_lowercase(), name.clone());
    }
    let mut names: Vec<String> = by_key.into_values().collect();
    names.sort();
    names
}

/// Renders language grants back into "Choose 2 from ..." / "Any 2" text.
pub fn describe_language_grants(grants: &[LanguageGrant]) -> String {
    grants
        .iter()
        .map(|grant| match grant {
            LanguageGrant::Fixed { languages } => {
                languages.iter().map(|l| title_case(l)).collect::<Vec<_>>().join(", ")
            }
            LanguageGrant::Choose { count, from } => {
                let options = from.iter().map(|l| title_case(l)).collect::<Vec<_>>().join(", ");
                format!("Choose {count} from {options}")
            }
            LanguageGrant::Any { count } => format!("Any {count}"),
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// "thieves' tools" -> "Thieves' Tools" (connecting words stay lowercase).
/// Idempotent on already-title-cased input (e.g. a real item name resolved
/// from the `items` table), so callers can apply it uniformly to both raw
/// import strings and DB-backed names.
pub fn title_case(name: &str) -> String {
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
#[path = "proficiency_tests.rs"]
mod tests;
