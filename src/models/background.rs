use crate::models::entry::Entry;
use crate::models::language::LanguageGrant;
use crate::models::proficiency::ToolGrant;
use crate::models::skill::SkillGrant;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Background {
    pub id: String,
    pub canonical_id: String,
    pub name: String,
    /// Short initials of the source book this background was imported from (e.g. "PHB").
    pub source: String,
    #[serde(default)]
    pub skills: Vec<SkillGrant>,
    #[serde(default)]
    pub languages: Vec<LanguageGrant>,
    #[serde(default)]
    pub tools: Vec<ToolGrant>,
    pub entries: Vec<Entry>,
}

/// Search/filter/sort parameters for listing backgrounds from the compendium.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BackgroundQuery {
    #[serde(default)]
    pub search: String,
    #[serde(default)]
    pub sources: Vec<String>,
    #[serde(default)]
    pub sort: BackgroundSort,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BackgroundSort {
    #[default]
    NameAsc,
    NameDesc,
    SourceAsc,
    SourceDesc,
}
