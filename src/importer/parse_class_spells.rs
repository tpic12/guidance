use serde::Deserialize;
use std::collections::HashMap;

/// Shape of 5etools' generated `gendata-spell-source-lookup.json`: spell
/// source (lowercase) -> spell name (lowercase) -> the classes/subclasses/etc.
/// that can access it. `class` and `subclass` are modeled — classVariant/
/// background/feat/race grants are still out of scope for this pass.
pub type RawSpellClassLookup = HashMap<String, HashMap<String, RawSpellClassEntry>>;

#[derive(Debug, Default, Deserialize)]
pub struct RawSpellClassEntry {
    /// Grant source -> class name -> details (usually `true`; the value
    /// itself is unused, only which class names are present matters).
    #[serde(default)]
    pub class: HashMap<String, HashMap<String, serde_json::Value>>,
    /// subclassSource -> className -> classSource -> subclassShortName ->
    /// details (again unused; only presence/keys matter).
    #[serde(default)]
    pub subclass: HashMap<String, HashMap<String, HashMap<String, HashMap<String, serde_json::Value>>>>,
}

pub fn parse_class_spell_lookup(raw: &str) -> anyhow::Result<RawSpellClassLookup> {
    Ok(serde_json::from_str(raw)?)
}

#[cfg(test)]
#[path = "parse_class_spells_tests.rs"]
mod tests;
