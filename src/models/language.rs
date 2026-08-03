use serde::{Deserialize, Serialize};

/// Mirrors `SkillGrant`: a granted language, or a slot the player must fill.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind")]
pub enum LanguageGrant {
    /// Languages granted outright, no player choice.
    Fixed { languages: Vec<String> },
    /// Choose `count` from a specific list of candidate languages.
    Choose { count: u8, from: Vec<String> },
    /// Choose `count` from all known languages ("any"/"anyStandard"/"other" in source data).
    Any { count: u8 },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LanguageType {
    Standard,
    Exotic,
    Rare,
    Secret,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Language {
    pub id: String,
    pub canonical_id: String,
    pub name: String,
    /// Short initials of the source book this language was imported from (e.g. "PHB").
    pub source: String,
    /// Absent for languages that only ever appear as a monster/racial trait (e.g. Aarakocra).
    pub language_type: Option<LanguageType>,
    pub script: Option<String>,
}
