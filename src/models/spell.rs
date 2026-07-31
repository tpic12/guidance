use crate::models::entry::Entry;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Spell {
    pub id: String,
    pub canonical_id: String,
    pub name: String,
    /// Short initials of the source book this spell was imported from (e.g. "PHB").
    pub source: String,
    pub level: u8,
    pub school: School,
    pub casting_time: u8,
    pub casting_type: CastingType,
    /// Trigger for reaction spells ("which you take when ..."), if any.
    pub casting_condition: Option<String>,
    pub range: Range,
    pub components: Components,
    pub duration: Duration,
    pub ritual: bool,
    pub description: Vec<Entry>,
    /// "At Higher Levels" text, if the spell can be upcast.
    #[serde(default)]
    pub higher_level: Vec<Entry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum School {
    Abjuration, Conjuration, Divination, Enchantment,
    Evocation, Illusion, Necromancy, Transmutation,
}

impl School {
    pub const ALL: [School; 8] = [
        School::Abjuration,
        School::Conjuration,
        School::Divination,
        School::Enchantment,
        School::Evocation,
        School::Illusion,
        School::Necromancy,
        School::Transmutation,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            School::Abjuration => "abjuration",
            School::Conjuration => "conjuration",
            School::Divination => "divination",
            School::Enchantment => "enchantment",
            School::Evocation => "evocation",
            School::Illusion => "illusion",
            School::Necromancy => "necromancy",
            School::Transmutation => "transmutation",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            School::Abjuration => "Abjuration",
            School::Conjuration => "Conjuration",
            School::Divination => "Divination",
            School::Enchantment => "Enchantment",
            School::Evocation => "Evocation",
            School::Illusion => "Illusion",
            School::Necromancy => "Necromancy",
            School::Transmutation => "Transmutation",
        }
    }
}

/// Unit of a spell's casting time, matching the distinctions the source data
/// makes (action vs. bonus action vs. reaction matters mechanically).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CastingType {
    Action, BonusAction, Reaction, Minute, Hour, Day, Week, Month
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Range {
    pub kind: String,
    pub distance: Option<u32>,
    pub unit: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Components {
    pub verbal: bool,
    pub somatic: bool,
    pub material: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Duration {
    pub kind: String,
    pub amount: Option<u32>,
    pub unit: Option<String>,
    pub concentration: bool,
}

/// Search/filter/sort parameters for listing spells from the compendium.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SpellQuery {
    #[serde(default)]
    pub search: String,
    #[serde(default)]
    pub schools: Vec<School>,
    #[serde(default)]
    pub levels: Vec<u8>,
    #[serde(default)]
    pub sources: Vec<String>,
    /// Class names (`classes.name`, via the `class_spells` link table) —
    /// spells not linked to any class (rare, or homebrew) are simply
    /// unreachable through this filter, same as an unlinked source.
    #[serde(default)]
    pub classes: Vec<String>,
    #[serde(default)]
    pub sort: SpellSort,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SpellSort {
    #[default]
    NameAsc,
    NameDesc,
    LevelAsc,
    LevelDesc,
    SchoolAsc,
    SchoolDesc,
}