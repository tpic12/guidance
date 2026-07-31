use crate::models::entry::Entry;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Feat {
    pub id: String,
    pub canonical_id: String,
    pub name: String,
    /// Short initials of the source book this feat was imported from (e.g. "PHB").
    pub source: String,
    /// Display text of the feat's prerequisites, if it has any.
    pub prerequisite: Option<String>,
    /// Display text of the ability score increase this feat grants, if any.
    pub ability: Option<String>,
    pub entries: Vec<Entry>,
}

/// Search/filter/sort parameters for listing feats from the compendium.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FeatQuery {
    #[serde(default)]
    pub search: String,
    #[serde(default)]
    pub sources: Vec<String>,
    #[serde(default)]
    pub sort: FeatSort,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FeatSort {
    #[default]
    NameAsc,
    NameDesc,
    SourceAsc,
    SourceDesc,
}
