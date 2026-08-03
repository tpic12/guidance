use crate::models::entry::Entry;
use crate::models::optional_feature::FeatureType;
use crate::models::proficiency::ToolGrant;
use crate::models::skill::SkillGrant;
use serde::{Deserialize, Serialize};

/// A character class as stored in the compendium (e.g. Barbarian).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Class {
    pub id: String,
    pub canonical_id: String,
    pub name: String,
    /// Short initials of the source book this class was imported from (e.g. "PHB").
    pub source: String,
    /// Hit die faces (12 for a d12).
    pub hit_die: u8,
    /// Ability codes for saving throw proficiencies, e.g. ["str", "con"].
    pub saving_throws: Vec<String>,
    /// What this class calls its subclasses, e.g. "Primal Path".
    pub subclass_title: String,
    /// Caster progression kind ("full", "1/2", "artificer", ...), if a caster.
    pub caster_progression: Option<String>,
    /// Ability code used for spellcasting, if a caster.
    pub spellcasting_ability: Option<String>,
    /// Fixed spells-known count per level (index 0 = level 1), if this class
    /// has a fixed "known spells" list (Sorcerer, Bard, ...) rather than
    /// preparing from a formula (Wizard, Paladin, ...).
    pub spells_known_progression: Option<Vec<u8>>,
    /// Fixed cantrips-known count per level (index 0 = level 1). `None` means
    /// no cantrips (correctly matches half-casters, who get none).
    pub cantrips_known_progression: Option<Vec<u8>>,
    pub proficiencies: Proficiencies,
    /// Proficiencies granted when this class is taken via multiclassing
    /// rather than as a character's first class — 5e only grants the full
    /// `proficiencies` above to a character's first class; every class taken
    /// after that only grants this smaller set (a subset of armor/weapons/
    /// tools, and usually 1 skill instead of several). Empty for classes
    /// whose imported source data doesn't carry a `multiclassing` block.
    #[serde(default)]
    pub multiclass_proficiencies: Proficiencies,
    /// Display-ready starting equipment lines.
    pub starting_equipment: Vec<String>,
    /// Extra column groups of the level-progression table (class-specific
    /// columns like Rages/Rage Damage, spell slots per level, ...).
    pub table_groups: Vec<ClassTableGroup>,
    /// Optional-feature pools this class grants picks from (Invocations,
    /// Fighting Style, ...), if any.
    #[serde(default)]
    pub optional_feature_progressions: Vec<OptionalFeatureProgression>,
    /// Minimum ability scores to multiclass into/out of this class. Outer
    /// `Vec` = OR'd alternatives; inner `Vec` = ability/score pairs all
    /// required within that alternative. Empty means no requirement set.
    #[serde(default)]
    pub multiclass_ability_prerequisites: Vec<Vec<(String, u8)>>,
}

/// How many picks of one `FeatureType` a class/subclass grants, per level
/// (index 0 = level 1) — same dense convention as `spells_known_progression`.
/// A class and its active subclass can both carry an entry for the *same*
/// type (e.g. Fighter grants a Fighting Style at 1st level, and its Champion
/// subclass grants a second one at 10th) — these are meant to be summed, not
/// treated as alternatives.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OptionalFeatureProgression {
    pub feature_type: FeatureType,
    pub known: Vec<u8>,
}

/// Starting proficiencies. Armor/weapons are pre-rendered to display strings;
/// tools and skills keep their structured fixed/choice/any shape (see
/// `models::proficiency::ToolGrant`/`models::skill::SkillGrant`) since
/// character creation needs to know which are actually choosable, not just
/// how to describe them.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Proficiencies {
    pub armor: Vec<String>,
    pub weapons: Vec<String>,
    #[serde(default)]
    pub tools: Vec<ToolGrant>,
    #[serde(default)]
    pub skills: Vec<SkillGrant>,
}

/// One column group of the class's level-progression table. Rows are
/// display-ready strings, one row per class level (1-20).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClassTableGroup {
    pub title: Option<String>,
    pub col_labels: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

/// A subclass (Primal Path, Divine Domain, ...) belonging to a class. The
/// four spellcasting fields mirror `Class`'s: most subclasses leave them
/// `None`, but some (Eldritch Knight, Arcane Trickster, ...) grant
/// spellcasting to an otherwise non-caster base class, in which case these
/// are the only place that data exists.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Subclass {
    pub id: String,
    pub class_id: String,
    pub name: String,
    pub short_name: String,
    pub source: String,
    pub caster_progression: Option<String>,
    pub spellcasting_ability: Option<String>,
    pub spells_known_progression: Option<Vec<u8>>,
    pub cantrips_known_progression: Option<Vec<u8>>,
    #[serde(default)]
    pub optional_feature_progressions: Vec<OptionalFeatureProgression>,
}

/// A leveled feature belonging to either a class or a subclass.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClassFeature {
    pub name: String,
    pub source: String,
    pub level: u8,
    pub entries: Vec<Entry>,
}

/// Row shape for the classes compendium index.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClassSummary {
    pub id: String,
    pub name: String,
    pub source: String,
    pub hit_die: u8,
    pub saving_throws: Vec<String>,
    pub subclass_count: u32,
    #[serde(default)]
    pub multiclass_ability_prerequisites: Vec<Vec<(String, u8)>>,
}

/// Everything needed to render a class detail page in one payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClassDetail {
    pub class: Class,
    pub features: Vec<ClassFeature>,
    pub subclasses: Vec<SubclassDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SubclassDetail {
    pub subclass: Subclass,
    pub features: Vec<ClassFeature>,
}

impl ClassDetail {
    /// Resolves a selected subclass id against this class's subclass list —
    /// used everywhere a character's effective spellcasting needs to
    /// consider a subclass-granted caster (Eldritch Knight, Arcane
    /// Trickster, ...) alongside its class.
    pub fn subclass(&self, subclass_id: Option<&str>) -> Option<&Subclass> {
        let subclass_id = subclass_id?;
        self.subclasses.iter().find(|sd| sd.subclass.id == subclass_id).map(|sd| &sd.subclass)
    }
}

/// Full name of an ability score code ("str" -> "Strength").
pub fn ability_label(code: &str) -> &str {
    match code {
        "str" => "Strength",
        "dex" => "Dexterity",
        "con" => "Constitution",
        "int" => "Intelligence",
        "wis" => "Wisdom",
        "cha" => "Charisma",
        other => other,
    }
}
