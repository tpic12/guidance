use crate::models::entry::Entry;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// A 5e "optional feature" — one of a pool of choices a class/subclass
/// grants a pick from (Eldritch Invocations, Metamagic, Maneuvers, Elemental
/// Disciplines, Infusions, Runes, Fighting Styles, Pact Boons, ...). Which
/// pool an entry belongs to is `feature_types`, not a separate table per
/// mechanic — 5etools itself models all of these as one content type
/// distinguished by a type code, and a single entry can belong to more than
/// one (Fighting Styles are shared across Fighter/Ranger/Paladin/Bard).
///
/// Character-creator gating (does this character qualify?) and granting
/// (some entries confer bonus spells) are explicitly out of scope for this
/// pass — see TDD.md's Phase 5 notes. `prerequisites` is kept here as
/// structured data (not pre-flattened to a display string like `Feat`'s own
/// prerequisite) precisely so that future work doesn't need to re-import.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OptionalFeature {
    pub id: String,
    pub canonical_id: String,
    pub name: String,
    /// Short initials of the source book this feature was imported from (e.g. "PHB").
    pub source: String,
    pub feature_types: Vec<FeatureType>,
    /// Each element is an alternative (OR); the fields set within one
    /// element are requirements that must all hold (AND). Empty means no
    /// prerequisite.
    #[serde(default)]
    pub prerequisites: Vec<PrerequisiteOption>,
    pub consumes: Option<ResourceCost>,
    pub entries: Vec<Entry>,
}

/// One AND-group of prerequisite requirements. Class/subclass are kept as
/// plain name (+ optional source) strings rather than resolved `Class`/
/// `Subclass` ids: the raw data references them by name only, subclass
/// `source` is frequently absent, and the character builder that will
/// actually check these already has the relevant `ClassDetail`/`Subclass`
/// loaded in memory to match by name against — no FK resolution gains
/// anything today. See TDD.md's Phase 5 notes.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct PrerequisiteOption {
    pub level: Option<u8>,
    pub class_name: Option<String>,
    pub class_source: Option<String>,
    pub subclass_name: Option<String>,
    pub subclass_source: Option<String>,
    /// Warlock pact boon name ("Blade", "Tome", "Chain", "Talisman"), if this
    /// option requires one.
    pub pact: Option<String>,
    /// Lowercased, tag-stripped spell names this option requires known.
    #[serde(default)]
    pub spells: Vec<String>,
    /// Freeform fallback for requirements this app can't otherwise model yet
    /// (e.g. a specific item, since there's no items table to FK into).
    pub other: Option<String>,
}

impl PrerequisiteOption {
    /// Combined class/subclass filter+display label ("Warlock", or
    /// "Fighter (Rune Knight)" when this option also names a subclass) —
    /// `None` if the option doesn't name a class at all. A bare class and a
    /// class+subclass combo are treated as distinct filter values rather
    /// than two independently-selectable axes, since a subclass without its
    /// class doesn't mean anything on its own.
    pub fn class_requirement_label(&self) -> Option<String> {
        let class = self.class_name.as_deref()?;
        Some(match &self.subclass_name {
            Some(subclass) => format!("{class} ({subclass})"),
            None => class.to_string(),
        })
    }
}

/// Distinct class/subclass requirement labels across every OR-alternative,
/// for populating `optional_feature_prerequisite_classes` at seed time.
pub fn class_requirement_labels(prerequisites: &[PrerequisiteOption]) -> Vec<String> {
    let mut labels: Vec<String> =
        prerequisites.iter().filter_map(PrerequisiteOption::class_requirement_label).collect();
    labels.sort();
    labels.dedup();
    labels
}

/// Distinct Warlock pact boon requirements across every OR-alternative, for
/// populating `optional_feature_prerequisite_pacts` at seed time.
pub fn required_pacts(prerequisites: &[PrerequisiteOption]) -> Vec<String> {
    let mut pacts: Vec<String> = prerequisites.iter().filter_map(|option| option.pact.clone()).collect();
    pacts.sort();
    pacts.dedup();
    pacts
}

/// Everything needed to check one `OptionalFeature`'s prerequisites against
/// an in-progress or already-saved character. Spell/feature-name sets are
/// expected pre-lowercased by the caller — this function does no
/// normalization of its own, matching how `PrerequisiteOption` itself stores
/// spell names lowercased.
pub struct EligibilityContext<'a> {
    pub level: u8,
    pub class_name: &'a str,
    pub class_source: &'a str,
    pub subclass_name: Option<&'a str>,
    pub subclass_source: Option<&'a str>,
    /// Lowercased names of every spell currently known/prepared/granted —
    /// callers must include subclass-auto-granted spells too, not just
    /// player-chosen ones, or a granted-spell-gated feature would wrongly
    /// read as ineligible.
    pub known_spell_names: &'a HashSet<String>,
    /// Lowercased names of optional features already chosen, across every
    /// feature type — today the only consumer is the Pact Boon cross-check
    /// (`"pact of the {pact}"`), kept general for future prerequisite kinds.
    pub chosen_feature_names: &'a HashSet<String>,
}

/// Whether at least one OR-alternative in `prerequisites` is fully
/// satisfied. Empty prerequisites means no requirement — always eligible.
pub fn is_eligible(prerequisites: &[PrerequisiteOption], ctx: &EligibilityContext) -> bool {
    prerequisites.is_empty() || prerequisites.iter().any(|option| option_satisfied(option, ctx))
}

fn option_satisfied(option: &PrerequisiteOption, ctx: &EligibilityContext) -> bool {
    if option.level.is_some_and(|level| ctx.level < level) {
        return false;
    }
    if let Some(class_name) = &option.class_name {
        if !class_name.eq_ignore_ascii_case(ctx.class_name) {
            return false;
        }
        if option.class_source.as_deref().is_some_and(|s| !s.eq_ignore_ascii_case(ctx.class_source)) {
            return false;
        }
    }
    if let Some(subclass_name) = &option.subclass_name {
        if !ctx.subclass_name.is_some_and(|active| active.eq_ignore_ascii_case(subclass_name)) {
            return false;
        }
        if option
            .subclass_source
            .as_deref()
            .is_some_and(|s| !ctx.subclass_source.is_some_and(|active| active.eq_ignore_ascii_case(s)))
        {
            return false;
        }
    }
    if let Some(pact) = &option.pact {
        let expected = format!("pact of the {}", pact.to_lowercase());
        if !ctx.chosen_feature_names.contains(&expected) {
            return false;
        }
    }
    if !option.spells.is_empty() && !option.spells.iter().any(|s| ctx.known_spell_names.contains(s)) {
        return false;
    }
    // `other` (freeform, e.g. a specific item) can't be checked — treated as
    // satisfied; `prerequisite_label()` still renders it so the player can
    // self-verify. Trusting the player here is better UX than never
    // offering an item-gated option at all.
    true
}

/// A resource some optional features spend to use (Sorcery Points,
/// Superiority Dice, an Arcane Shot use, ...) — display-only for now.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourceCost {
    pub name: String,
    pub amount: Option<u32>,
    pub amount_min: Option<u32>,
    pub amount_max: Option<u32>,
}

/// 5etools' `featureType` codes — which pool of choices an entry belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeatureType {
    EldritchInvocation,
    PactBoon,
    Metamagic,
    ArtificerInfusion,
    Maneuver,
    ArcaneShot,
    ElementalDiscipline,
    Rune,
    FightingStyleFighter,
    FightingStyleRanger,
    FightingStylePaladin,
    FightingStyleBard,
}

impl FeatureType {
    pub const ALL: [FeatureType; 12] = [
        FeatureType::EldritchInvocation,
        FeatureType::PactBoon,
        FeatureType::Metamagic,
        FeatureType::ArtificerInfusion,
        FeatureType::Maneuver,
        FeatureType::ArcaneShot,
        FeatureType::ElementalDiscipline,
        FeatureType::Rune,
        FeatureType::FightingStyleFighter,
        FeatureType::FightingStyleRanger,
        FeatureType::FightingStylePaladin,
        FeatureType::FightingStyleBard,
    ];

    /// Maps a 5etools `featureType` code; `None` for codes this app doesn't
    /// model (tolerated, not a hard import error — see transform).
    pub fn from_code(code: &str) -> Option<Self> {
        Some(match code {
            "EI" => FeatureType::EldritchInvocation,
            "PB" => FeatureType::PactBoon,
            "MM" => FeatureType::Metamagic,
            "AI" => FeatureType::ArtificerInfusion,
            "MV:B" => FeatureType::Maneuver,
            "AS" => FeatureType::ArcaneShot,
            "ED" => FeatureType::ElementalDiscipline,
            "RN" => FeatureType::Rune,
            "FS:F" => FeatureType::FightingStyleFighter,
            "FS:R" => FeatureType::FightingStyleRanger,
            "FS:P" => FeatureType::FightingStylePaladin,
            "FS:B" => FeatureType::FightingStyleBard,
            _ => return None,
        })
    }

    /// Stable snake_case key used for DB storage/query binding (distinct
    /// from `from_code`'s 5etools-shaped 2-letter codes, and from `label`'s
    /// display text) — mirrors `School::as_str` in `src/models/spell.rs`.
    pub fn as_str(&self) -> &'static str {
        match self {
            FeatureType::EldritchInvocation => "eldritch_invocation",
            FeatureType::PactBoon => "pact_boon",
            FeatureType::Metamagic => "metamagic",
            FeatureType::ArtificerInfusion => "artificer_infusion",
            FeatureType::Maneuver => "maneuver",
            FeatureType::ArcaneShot => "arcane_shot",
            FeatureType::ElementalDiscipline => "elemental_discipline",
            FeatureType::Rune => "rune",
            FeatureType::FightingStyleFighter => "fighting_style_fighter",
            FeatureType::FightingStyleRanger => "fighting_style_ranger",
            FeatureType::FightingStylePaladin => "fighting_style_paladin",
            FeatureType::FightingStyleBard => "fighting_style_bard",
        }
    }

    pub fn from_str_key(key: &str) -> Option<Self> {
        FeatureType::ALL.into_iter().find(|feature_type| feature_type.as_str() == key)
    }

    pub fn label(&self) -> &'static str {
        match self {
            FeatureType::EldritchInvocation => "Eldritch Invocation (Warlock)",
            FeatureType::PactBoon => "Pact Boon (Warlock)",
            FeatureType::Metamagic => "Metamagic (Sorcerer)",
            FeatureType::ArtificerInfusion => "Infusion (Artificer)",
            FeatureType::Maneuver => "Maneuver (Battle Master)",
            FeatureType::ArcaneShot => "Arcane Shot (Arcane Archer)",
            FeatureType::ElementalDiscipline => "Elemental Discipline (Monk)",
            FeatureType::Rune => "Rune (Rune Knight)",
            FeatureType::FightingStyleFighter => "Fighting Style (Fighter)",
            FeatureType::FightingStyleRanger => "Fighting Style (Ranger)",
            FeatureType::FightingStylePaladin => "Fighting Style (Paladin)",
            FeatureType::FightingStyleBard => "Fighting Style (Bard)",
        }
    }

    /// Same as `label`, minus the "which class grants this" parenthetical —
    /// for the compendium browse page, where that distinction doesn't matter
    /// (unlike character creation, which uses `label` because it does: it
    /// gates what a specific class can actually pick).
    pub fn group_label(&self) -> &'static str {
        match self {
            FeatureType::EldritchInvocation => "Eldritch Invocation",
            FeatureType::PactBoon => "Pact Boon",
            FeatureType::Metamagic => "Metamagic",
            FeatureType::ArtificerInfusion => "Infusion",
            FeatureType::Maneuver => "Maneuver",
            FeatureType::ArcaneShot => "Arcane Shot",
            FeatureType::ElementalDiscipline => "Elemental Discipline",
            FeatureType::Rune => "Rune",
            FeatureType::FightingStyleFighter
            | FeatureType::FightingStyleRanger
            | FeatureType::FightingStylePaladin
            | FeatureType::FightingStyleBard => "Fighting Style",
        }
    }
}

/// Search/filter/sort parameters for listing optional features from the compendium.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct OptionalFeatureQuery {
    #[serde(default)]
    pub search: String,
    #[serde(default)]
    pub types: Vec<FeatureType>,
    #[serde(default)]
    pub sources: Vec<String>,
    /// Matches `PrerequisiteOption::class_requirement_label` values (see
    /// `optional_feature_prerequisite_classes`).
    #[serde(default)]
    pub class_requirements: Vec<String>,
    /// Matches `PrerequisiteOption::pact` values (see
    /// `optional_feature_prerequisite_pacts`).
    #[serde(default)]
    pub pacts: Vec<String>,
    #[serde(default)]
    pub sort: OptionalFeatureSort,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum OptionalFeatureSort {
    #[default]
    NameAsc,
    NameDesc,
    SourceAsc,
    SourceDesc,
}

/// Renders a feature's prerequisites as display text — a derived value
/// recomputed from the structured data, not stored (see `TDD.md` / the
/// module doc comment above on why `prerequisites` stays structured).
/// Mirrors `transform_feat.rs`'s `prerequisite_label`/`prerequisite_option_parts`:
/// options join with " or ", requirements within an option join with ", ".
pub fn prerequisite_label(prerequisites: &[PrerequisiteOption]) -> Option<String> {
    let options: Vec<String> = prerequisites
        .iter()
        .filter_map(|option| {
            let parts = prerequisite_option_parts(option);
            if parts.is_empty() {
                None
            } else {
                Some(parts.join(", "))
            }
        })
        .collect();
    if options.is_empty() {
        None
    } else {
        Some(options.join(" or "))
    }
}

fn prerequisite_option_parts(option: &PrerequisiteOption) -> Vec<String> {
    let mut parts = Vec::new();

    if let Some(level) = option.level {
        let class = option.class_name.as_deref().unwrap_or("character");
        let subclass = option.subclass_name.as_deref();
        parts.push(match subclass {
            Some(subclass) => format!("{class} ({subclass}) level {level}"),
            None => format!("{class} level {level}"),
        });
    } else if let Some(class) = &option.class_name {
        parts.push(class.clone());
    }
    if let Some(pact) = &option.pact {
        parts.push(format!("Pact of the {pact}"));
    }
    if !option.spells.is_empty() {
        parts.push(format!("{} known", option.spells.join(" or ")));
    }
    if let Some(other) = &option.other {
        parts.push(other.clone());
    }

    parts
}

#[cfg(test)]
#[path = "optional_feature_tests.rs"]
mod tests;
