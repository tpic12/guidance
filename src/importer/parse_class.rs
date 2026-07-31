use serde::Deserialize;
use std::collections::HashMap;

/// One 5etools-shaped class file: a single class plus its subclasses and the
/// flattened feature text for both.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawClassFile {
    #[serde(default)]
    pub class: Vec<RawClass>,
    #[serde(default)]
    pub subclass: Vec<RawSubclass>,
    #[serde(default)]
    pub class_feature: Vec<RawClassFeature>,
    #[serde(default)]
    pub subclass_feature: Vec<RawSubclassFeature>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawClass {
    pub name: String,
    #[serde(default)]
    pub source: String,
    pub hd: RawHitDie,
    /// Saving throw proficiency ability codes, e.g. ["str", "con"].
    #[serde(default)]
    pub proficiency: Vec<String>,
    #[serde(default)]
    pub starting_proficiencies: RawStartingProficiencies,
    /// Proficiencies gained when this class is taken via multiclassing
    /// (a smaller subset of `starting_proficiencies` per 5e's multiclass
    /// rules) — absent for classes whose source data doesn't carry it.
    #[serde(default)]
    pub multiclassing: Option<RawMulticlassing>,
    pub starting_equipment: Option<RawStartingEquipment>,
    #[serde(default)]
    pub class_table_groups: Vec<RawTableGroup>,
    pub subclass_title: Option<String>,
    pub caster_progression: Option<String>,
    pub spellcasting_ability: Option<String>,
    #[serde(default, rename = "spellsKnownProgression")]
    pub spells_known_progression: Option<Vec<u8>>,
    #[serde(default, rename = "cantripProgression")]
    pub cantrips_known_progression: Option<Vec<u8>>,
    /// How many optional features (Invocations, Fighting Style, ...) this
    /// class grants picks of, per level — real key is lowercase-`f`
    /// `optionalfeatureProgression`, unlike the other progressions above.
    #[serde(default, rename = "optionalfeatureProgression")]
    pub optional_feature_progression: Vec<RawOptionalFeatureProgressionEntry>,
}

#[derive(Debug, Deserialize)]
pub struct RawHitDie {
    pub number: u8,
    pub faces: u8,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawMulticlassing {
    #[serde(default)]
    pub proficiencies_gained: Option<RawStartingProficiencies>,
    #[serde(default)]
    pub requirements: Option<RawMulticlassRequirements>,
}

/// Ability-score prerequisite shape: flat keys are ANDed; a lone `or`
/// element's keys are alternatives instead (see `transform_class.rs`).
#[derive(Debug, Deserialize)]
pub struct RawMulticlassRequirements {
    #[serde(default)]
    pub or: Option<Vec<HashMap<String, u8>>>,
    #[serde(flatten)]
    pub flat: HashMap<String, u8>,
}

#[derive(Debug, Default, Deserialize)]
pub struct RawStartingProficiencies {
    #[serde(default)]
    pub armor: Vec<serde_json::Value>,
    #[serde(default)]
    pub weapons: Vec<serde_json::Value>,
    #[serde(default)]
    pub tools: Vec<serde_json::Value>,
    #[serde(default)]
    pub skills: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawStartingEquipment {
    #[serde(default)]
    pub default: Vec<String>,
    pub gold_alternative: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawTableGroup {
    pub title: Option<String>,
    #[serde(default)]
    pub col_labels: Vec<String>,
    #[serde(default)]
    pub rows: Vec<Vec<serde_json::Value>>,
    /// Spell-slot groups use this key instead of `rows`; cells are slot counts.
    #[serde(default)]
    pub rows_spell_progression: Vec<Vec<serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawSubclass {
    pub name: String,
    pub short_name: String,
    #[serde(default)]
    pub source: String,
    pub class_name: String,
    #[serde(default)]
    pub class_source: String,
    pub caster_progression: Option<String>,
    pub spellcasting_ability: Option<String>,
    #[serde(default, rename = "spellsKnownProgression")]
    pub spells_known_progression: Option<Vec<u8>>,
    #[serde(default, rename = "cantripProgression")]
    pub cantrips_known_progression: Option<Vec<u8>>,
    /// Spells this subclass auto-grants at specific levels (Cleric domain
    /// spells, Druid circle spells, ...) — distinct from the class/subclass's
    /// *selectable* spell list (`class_spells`/`subclass_spells`), which
    /// comes from a different file entirely (see `parse_class_spells.rs`).
    #[serde(default, rename = "additionalSpells")]
    pub additional_spells: Vec<RawAdditionalSpells>,
    /// See `RawClass::optional_feature_progression` — subclasses (Battle
    /// Master's Maneuvers, Champion's second Fighting Style, ...) grant
    /// these independently of their base class.
    #[serde(default, rename = "optionalfeatureProgression")]
    pub optional_feature_progression: Vec<RawOptionalFeatureProgressionEntry>,
}

/// One `optionalfeatureProgression` entry: how many picks of a given
/// `featureType` (or types — always singular in practice, but modeled as a
/// list to match the raw shape) this class/subclass grants, per level.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawOptionalFeatureProgressionEntry {
    #[serde(default)]
    pub feature_type: Vec<String>,
    pub progression: RawOptionalFeatureProgressionShape,
}

/// 5etools encodes this two different ways depending on how often the count
/// changes: a dense array (index 0 = level 1, same convention as
/// `spellsKnownProgression`) when it changes almost every level (e.g.
/// Warlock Invocations), or a sparse `{level: cumulative_count}` map when it
/// only changes at a few breakpoints (e.g. `{"3": 2, "10": 3, "17": 4}` for
/// Sorcerer Metamagic) — `transform_class.rs` normalizes both to the dense
/// shape.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum RawOptionalFeatureProgressionShape {
    Dense(Vec<u8>),
    Sparse(HashMap<String, u8>),
}

/// One `additionalSpells` entry. `name` is present only for subclasses whose
/// grants are split into named sub-choice variants (e.g. Druid Circle of the
/// Land's terrains) — those get fanned out into separate subclasses by
/// `transform_class.rs` rather than modeled as a player-facing sub-choice.
/// `prepared`/`known` map a level (as a string key) to either a flat array of
/// spell-name strings, or some other shape (a `{"choose": ...}` filter, or a
/// `{"daily": {...}}` wrapper) that isn't a fixed always-active grant and
/// gets skipped by the transform step.
#[derive(Debug, Default, Deserialize)]
pub struct RawAdditionalSpells {
    pub name: Option<String>,
    #[serde(default)]
    pub prepared: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub known: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawClassFeature {
    pub name: String,
    #[serde(default)]
    pub source: String,
    pub class_name: String,
    pub level: u8,
    #[serde(default)]
    pub entries: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawSubclassFeature {
    pub name: String,
    #[serde(default)]
    pub source: String,
    pub class_name: String,
    pub subclass_short_name: String,
    #[serde(default)]
    pub subclass_source: String,
    pub level: u8,
    #[serde(default)]
    pub entries: Vec<serde_json::Value>,
}

pub fn parse_class_file(raw: &str) -> anyhow::Result<RawClassFile> {
    Ok(serde_json::from_str(raw)?)
}
