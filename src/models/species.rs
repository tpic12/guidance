use crate::models::entry::Entry;
use crate::models::language::LanguageGrant;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Species {
    pub id: String,
    pub canonical_id: String,
    pub name: String,
    /// Short initials of the source book this species was imported from (e.g. "PHB").
    pub source: String,
    /// Display text of the ability score increases, e.g. "Dexterity +2, Wisdom +1".
    pub ability: Option<String>,
    /// Structured form of the same grants `ability` describes, for the
    /// character builder to actually apply — `ability` stays display-only
    /// since compendium pages/the species picker only ever render it as text.
    #[serde(default)]
    pub ability_bonuses: Vec<AbilityBonusGrant>,
    /// Display text of the size category, e.g. "Medium" or "Small or Medium".
    pub size: Option<String>,
    /// Display text of the movement speeds, e.g. "30 ft., fly 50 ft.".
    pub speed: Option<String>,
    /// Darkvision range in feet, if the species has it.
    pub darkvision: Option<u32>,
    #[serde(default)]
    pub languages: Vec<LanguageGrant>,
    pub entries: Vec<Entry>,
}

/// One ability-bonus grant a species offers, preserving the fixed/choice
/// distinction `ability`'s flattened display string can't — mirrors
/// `SkillGrant` in `models/skill.rs`. A species normally has 0-2 grants
/// (e.g. one `Fixed` and one `Choose`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind")]
pub enum AbilityBonusGrant {
    /// A specific ability increased by a fixed amount, no player choice.
    /// `amount` can be negative (e.g. Bugbear's Intelligence -2).
    Fixed { code: String, amount: i8 },
    /// Choose `count` abilities from `from` (or any of the six if `from` is
    /// empty/covers all six — see `is_free_ability_choice`) to each receive
    /// `amount`.
    Choose { count: u8, amount: i8, from: Vec<String> },
}

/// True if `from` means "choose from all six abilities" — the raw data
/// sometimes lists all six explicitly, sometimes omits `from` entirely for
/// the same free choice, mirroring the importer's existing `n == 0 || n >=
/// 6` convention for this ambiguity.
pub fn is_free_ability_choice(from: &[String]) -> bool {
    from.is_empty() || from.len() >= 6
}

/// Search/filter/sort parameters for listing species from the compendium.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SpeciesQuery {
    #[serde(default)]
    pub search: String,
    #[serde(default)]
    pub sources: Vec<String>,
    #[serde(default)]
    pub sort: SpeciesSort,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SpeciesSort {
    #[default]
    NameAsc,
    NameDesc,
    SourceAsc,
    SourceDesc,
}
