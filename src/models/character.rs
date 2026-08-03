use crate::models::background::Background;
use crate::models::class::{ability_label, Class, ClassDetail, ClassFeature, Subclass};
use crate::models::feat::Feat;
use crate::models::language::LanguageGrant;
use crate::models::optional_feature::{FeatureType, OptionalFeature};
use crate::models::proficiency::{contains_ignore_case, ToolCategory, ToolGrant, ToolOption};
use crate::models::skill::{is_valid_skill, SkillGrant, SKILLS};
use crate::models::species::{AbilityBonusGrant, Species};
use crate::models::spell::Spell;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// One class a character has levels in, and the subclass chosen within it
/// (if that class's own level has reached its subclass-unlock level). `id`s
/// are FKs into the `classes`/`subclasses` reference tables, never copies.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClassLevel {
    pub class_id: String,
    pub subclass_id: Option<String>,
    pub level: u8,
}

/// A character's classes: `classes[0]` is implicitly the primary/first
/// class (the order classes were added in), which is what 5e's multiclass
/// rules key off (e.g. only the first class grants full starting skill
/// proficiencies) — there's no separate "primary" flag to keep in sync.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Character {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub species_id: String,
    pub background_id: String,
    pub classes: Vec<ClassLevel>,
    pub ability_method: AbilityMethod,
    pub abilities: AbilityScores,
    #[serde(default)]
    pub asi_choices: Vec<Option<AsiChoice>>,
    /// Skills chosen to fill the class/background's "choose N"/"any N"
    /// slots. Fixed grants aren't stored here — like `saving_throws`,
    /// they're re-derived from class/background data via `skill_slots`
    /// whenever needed.
    #[serde(default)]
    pub skill_choices: Vec<String>,
    /// Skills chosen for expertise (double proficiency bonus). Each entry
    /// must already be a proficient skill (fixed grant or `skill_choices`).
    #[serde(default)]
    pub expertise_choices: Vec<String>,
    /// Languages chosen to fill the background/species's "choose N"/"any N"
    /// slots. Fixed grants aren't stored here — re-derived via `language_slots`.
    #[serde(default)]
    pub language_choices: Vec<String>,
    /// Tools chosen to fill the class/background's "choose N"/"any N"/
    /// category slots. Fixed grants aren't stored here — re-derived via `tool_slots`.
    #[serde(default)]
    pub tool_choices: Vec<String>,
    /// Skill proficiencies the player added directly, unattached to any
    /// class/background/species grant — an escape valve for prose-only
    /// grants the import pipeline can't parse (see ticket #66).
    #[serde(default)]
    pub custom_skill_proficiencies: Vec<String>,
    /// Language proficiencies the player added directly — see
    /// `custom_skill_proficiencies`.
    #[serde(default)]
    pub custom_language_proficiencies: Vec<String>,
    /// Tool proficiencies the player added directly — see
    /// `custom_skill_proficiencies`.
    #[serde(default)]
    pub custom_tool_proficiencies: Vec<String>,
    /// Cantrips known, by spell id. Always empty for non-casters.
    #[serde(default)]
    pub cantrip_choices: Vec<String>,
    /// Spells known/prepared, by spell id. Always empty for non-casters.
    #[serde(default)]
    pub spell_choices: Vec<String>,
    /// Spells picked to fill a subclass's `{"choose": ...}` filter grants
    /// (e.g. Cleric Nature Domain's "any Druid cantrip").
    #[serde(default)]
    pub spell_grant_choices: Vec<String>,
    /// Optional features chosen (Invocations, Fighting Style, Maneuvers,
    /// ...), by `OptionalFeature` id. Always empty for classes/subclasses
    /// that don't grant any.
    #[serde(default)]
    pub optional_feature_choices: Vec<String>,
    /// Ability codes chosen to receive a species' "choose N" bonus grant, if
    /// it has one (empty otherwise). Assumes at most one `Choose` grant per
    /// species — true of every species in this app's data today. Only
    /// meaningful when `ability_bonus_source` is `Species`; must be empty
    /// when `Custom`.
    #[serde(default)]
    pub species_ability_choices: Vec<String>,
    /// Whether the species' own bonus grants apply, or the player picked a
    /// custom split instead (needed since some imported species define no
    /// bonuses at all).
    #[serde(default)]
    pub ability_bonus_source: AbilityBonusSource,
    /// Exactly two *different* ability codes when `ability_bonus_source` is
    /// `Custom`: the first gains +2, the second +1 — unlike
    /// `AsiChoice::Abilities`, repeating a code isn't allowed here. Empty
    /// (and ignored) when `Species`.
    #[serde(default)]
    pub custom_ability_bonus_choices: Vec<String>,
    #[serde(default)]
    pub hp_method: HpMethod,
    /// Per-level hit die rolls for `HpMethod::Rolled`: one entry per level
    /// 2..=level (level 1 is always the hit die's max, never rolled, so it
    /// isn't stored here). Constitution modifier is added at resolve time —
    /// see `resolved_hp_max` — not baked into these values.
    #[serde(default)]
    pub hp_rolls: Vec<u8>,
    /// Flat HP total for `HpMethod::Manual`, constitution modifier already
    /// included by the player.
    #[serde(default)]
    pub hp_manual: Option<i32>,
    /// Whether classes must meet `Class::multiclass_ability_prerequisites`.
    #[serde(default = "default_true")]
    pub enforce_multiclass_prereqs: bool,
}

fn default_true() -> bool {
    true
}

impl Character {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("Name is required".to_string());
        }
        if self.classes.is_empty() {
            return Err("Choose a class".to_string());
        }
        for entry in &self.classes {
            if entry.class_id.is_empty() {
                return Err("Choose a class".to_string());
            }
            if !(1..=20).contains(&entry.level) {
                return Err("Each class's level must be between 1 and 20".to_string());
            }
        }
        let mut seen_classes = HashSet::new();
        for entry in &self.classes {
            if !seen_classes.insert(&entry.class_id) {
                return Err(format!("'{}' was chosen as a class more than once", entry.class_id));
            }
        }
        if !(1..=20).contains(&total_level(&self.classes)) {
            return Err("Total level must be between 1 and 20".to_string());
        }
        if self.species_id.is_empty() {
            return Err("Choose a species".to_string());
        }
        if self.background_id.is_empty() {
            return Err("Choose a background".to_string());
        }
        for code in ABILITY_CODES {
            if !(1..=20).contains(&self.abilities.get(code)) {
                return Err(format!("{code} must be between 1 and 20"));
            }
        }
        for choice in self.asi_choices.iter().flatten() {
            match choice {
                AsiChoice::Feat { feat_id } => {
                    if feat_id.trim().is_empty() {
                        return Err("A feat choice must name a feat".to_string());
                    }
                }
                AsiChoice::Abilities { codes } => {
                    if codes.len() != 2 || codes.iter().any(|code| !ABILITY_CODES.contains(&code.as_str()))
                    {
                        return Err("An ability increase choice must name two valid abilities".to_string());
                    }
                }
            }
        }
        for skill in self.skill_choices.iter().chain(self.expertise_choices.iter()) {
            if !is_valid_skill(skill) {
                return Err(format!("'{skill}' is not a valid skill"));
            }
        }
        let mut seen = HashSet::new();
        for skill in &self.skill_choices {
            if !seen.insert(skill) {
                return Err(format!("'{skill}' was chosen more than once"));
            }
        }
        let mut seen = HashSet::new();
        for skill in &self.expertise_choices {
            if !seen.insert(skill) {
                return Err(format!("'{skill}' was chosen for expertise more than once"));
            }
        }
        let mut seen = HashSet::new();
        for language in &self.language_choices {
            if !seen.insert(language) {
                return Err(format!("'{language}' was chosen more than once"));
            }
        }
        let mut seen = HashSet::new();
        for tool in &self.tool_choices {
            if !seen.insert(tool) {
                return Err(format!("'{tool}' was chosen more than once"));
            }
        }
        for skill in &self.custom_skill_proficiencies {
            if !is_valid_skill(skill) {
                return Err(format!("'{skill}' is not a valid skill"));
            }
        }
        let mut seen = HashSet::new();
        for skill in &self.custom_skill_proficiencies {
            if !seen.insert(skill) {
                return Err(format!("'{skill}' was added as a custom proficiency more than once"));
            }
        }
        let mut seen = HashSet::new();
        for language in &self.custom_language_proficiencies {
            if !seen.insert(language) {
                return Err(format!("'{language}' was added as a custom proficiency more than once"));
            }
        }
        let mut seen = HashSet::new();
        for tool in &self.custom_tool_proficiencies {
            if !seen.insert(tool) {
                return Err(format!("'{tool}' was added as a custom proficiency more than once"));
            }
        }
        match self.ability_bonus_source {
            AbilityBonusSource::Species => {
                if !self.custom_ability_bonus_choices.is_empty() {
                    return Err(
                        "Custom ability bonus choices must be empty when using the species' bonus"
                            .to_string(),
                    );
                }
                for code in &self.species_ability_choices {
                    if !ABILITY_CODES.contains(&code.as_str()) {
                        return Err(format!("'{code}' is not a valid ability"));
                    }
                }
                let mut seen = HashSet::new();
                for code in &self.species_ability_choices {
                    if !seen.insert(code) {
                        return Err(format!("'{code}' was chosen more than once for a species bonus"));
                    }
                }
            }
            AbilityBonusSource::Custom => {
                if !self.species_ability_choices.is_empty() {
                    return Err(
                        "Species ability bonus choices must be empty when using a custom bonus"
                            .to_string(),
                    );
                }
                if self.custom_ability_bonus_choices.len() != 2
                    || self
                        .custom_ability_bonus_choices
                        .iter()
                        .any(|code| !ABILITY_CODES.contains(&code.as_str()))
                    || self.custom_ability_bonus_choices[0] == self.custom_ability_bonus_choices[1]
                {
                    return Err(
                        "A custom ability bonus must name two different abilities (the first gains +2, the second +1)"
                            .to_string(),
                    );
                }
            }
        }
        let mut seen = HashSet::new();
        for spell_id in &self.cantrip_choices {
            if !seen.insert(spell_id) {
                return Err(format!("'{spell_id}' was chosen more than once"));
            }
        }
        let mut seen = HashSet::new();
        for spell_id in &self.spell_choices {
            if !seen.insert(spell_id) {
                return Err(format!("'{spell_id}' was chosen more than once"));
            }
        }
        let mut seen = HashSet::new();
        for feature_id in &self.optional_feature_choices {
            if !seen.insert(feature_id) {
                return Err(format!("'{feature_id}' was chosen more than once"));
            }
        }
        match self.hp_method {
            HpMethod::Average => {}
            HpMethod::Rolled => {
                if self.hp_rolls.len() != total_level(&self.classes).saturating_sub(1) as usize {
                    return Err("Rolled HP must include one entry for every level after 1st".to_string());
                }
                if self.hp_rolls.iter().any(|&roll| roll == 0) {
                    return Err("Each rolled HP value must be at least 1".to_string());
                }
            }
            HpMethod::Manual => {
                if !self.hp_manual.is_some_and(|hp| hp > 0) {
                    return Err("Manual HP must be a positive number".to_string());
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AbilityMethod {
    #[default]
    StandardArray,
    PointBuy,
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum HpMethod {
    #[default]
    Average,
    Rolled,
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AbilityBonusSource {
    #[default]
    Species,
    Custom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct AbilityScores {
    pub strength: u8,
    pub dexterity: u8,
    pub constitution: u8,
    pub intelligence: u8,
    pub wisdom: u8,
    pub charisma: u8,
}

impl Default for AbilityScores {
    fn default() -> Self {
        Self {
            strength: 10,
            dexterity: 10,
            constitution: 10,
            intelligence: 10,
            wisdom: 10,
            charisma: 10,
        }
    }
}

pub const ABILITY_CODES: [&str; 6] = ["str", "dex", "con", "int", "wis", "cha"];

impl AbilityScores {
    pub fn get(&self, code: &str) -> u8 {
        match code {
            "str" => self.strength,
            "dex" => self.dexterity,
            "con" => self.constitution,
            "int" => self.intelligence,
            "wis" => self.wisdom,
            "cha" => self.charisma,
            _ => 0,
        }
    }

    pub fn set(&mut self, code: &str, value: u8) {
        match code {
            "str" => self.strength = value,
            "dex" => self.dexterity = value,
            "con" => self.constitution = value,
            "int" => self.intelligence = value,
            "wis" => self.wisdom = value,
            "cha" => self.charisma = value,
            _ => {}
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind")]
pub enum AsiChoice {
    Feat { feat_id: String },
    Abilities { codes: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CharacterSummary {
    pub id: String,
    pub name: String,
    pub level: u8,
    pub class_name: Option<String>,
    pub species_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CharacterSheet {
    pub character: Character,
    pub species: Option<Species>,
    pub background: Option<Background>,
    /// One resolved `ClassDetail` per `character.classes` entry, same index
    /// order — a purged/archived class becomes a gap (see `db::get_character_sheet`),
    /// not a whole-sheet failure.
    pub classes: Vec<ClassDetail>,
    pub feats: Vec<Feat>,
    pub cantrips: Vec<Spell>,
    pub spells: Vec<Spell>,
    /// Spells the active subclass auto-grants at the character's current
    /// level (Cleric domain spells, Druid circle spells, ...) — always
    /// prepared, never part of `spell_choices`/`cantrip_choices`.
    pub granted_spells: Vec<Spell>,
    /// Optional features chosen (Invocations, Fighting Style, ...), resolved
    /// from `optional_feature_choices`.
    pub optional_features: Vec<OptionalFeature>,
}

pub const STANDARD_ARRAY: [u8; 6] = [15, 14, 13, 12, 10, 8];
pub const POINT_BUY_BUDGET: u8 = 27;

pub fn point_buy_cost(score: u8) -> Option<u8> {
    match score {
        8 => Some(0),
        9 => Some(1),
        10 => Some(2),
        11 => Some(3),
        12 => Some(4),
        13 => Some(5),
        14 => Some(7),
        15 => Some(9),
        _ => None,
    }
}

pub fn ability_modifier(score: u8) -> i8 {
    (score as i8 - 10).div_euclid(2)
}

/// A character's total level, summed across every class — the single source
/// of truth `Character` itself no longer stores a redundant `level` field for.
pub fn total_level(classes: &[ClassLevel]) -> u8 {
    classes.iter().map(|entry| entry.level).sum()
}

pub fn proficiency_bonus(level: u8) -> u8 {
    2 + (level.saturating_sub(1)) / 4
}

/// Applies a species' ability-bonus grants on top of a base set of scores —
/// the shared derivation used by both the wizard's live preview and
/// `final_abilities`, so the two can never disagree. `choices` is only
/// consulted for `Choose` grants; `Fixed` grants always apply.
pub fn apply_species_ability_bonus(
    scores: AbilityScores,
    species: Option<&Species>,
    choices: &[String],
) -> AbilityScores {
    let mut scores = scores;
    let Some(species) = species else { return scores };
    for grant in &species.ability_bonuses {
        match grant {
            AbilityBonusGrant::Fixed { code, amount } => {
                let bumped = (scores.get(code) as i16 + *amount as i16).clamp(1, 20) as u8;
                scores.set(code, bumped);
            }
            AbilityBonusGrant::Choose { amount, from, .. } => {
                for code in choices {
                    if crate::models::species::is_free_ability_choice(from) || from.contains(code) {
                        let bumped = (scores.get(code) as i16 + *amount as i16).clamp(1, 20) as u8;
                        scores.set(code, bumped);
                    }
                }
            }
        }
    }
    scores
}

/// Applies a flat "+1 per code" bump (same code twice = +2), the shape
/// shared by `AsiChoice::Abilities` and a custom species ability bonus.
pub fn apply_ability_choice_bumps(scores: AbilityScores, codes: &[String]) -> AbilityScores {
    let mut scores = scores;
    for code in codes {
        scores.set(code, (scores.get(code) + 1).min(20));
    }
    scores
}

/// Applies a custom species ability bonus: +2 to the first code, +1 to the
/// second. Unlike `apply_ability_choice_bumps` (ASI's "+1 each, same twice
/// for +2"), the two codes are expected to differ — see
/// `Character::validate`.
pub fn apply_custom_species_bonus(scores: AbilityScores, codes: &[String]) -> AbilityScores {
    let mut scores = scores;
    if let [primary, secondary] = codes {
        scores.set(primary, (scores.get(primary) + 2).min(20));
        scores.set(secondary, (scores.get(secondary) + 1).min(20));
    }
    scores
}

/// Applies either the species' own bonus grants or a player-picked custom
/// split, depending on `source` — the shared derivation used by both the
/// wizard's live preview and `final_abilities`, so the two can never
/// disagree.
pub fn resolve_ability_bonus(
    scores: AbilityScores,
    species: Option<&Species>,
    source: AbilityBonusSource,
    species_choices: &[String],
    custom_choices: &[String],
) -> AbilityScores {
    match source {
        AbilityBonusSource::Species => apply_species_ability_bonus(scores, species, species_choices),
        AbilityBonusSource::Custom => apply_custom_species_bonus(scores, custom_choices),
    }
}

pub fn final_abilities(character: &Character, species: Option<&Species>) -> [(&'static str, u8); 6] {
    let mut scores = resolve_ability_bonus(
        character.abilities,
        species,
        character.ability_bonus_source,
        &character.species_ability_choices,
        &character.custom_ability_bonus_choices,
    );
    for choice in character.asi_choices.iter().flatten() {
        if let AsiChoice::Abilities { codes } = choice {
            scores = apply_ability_choice_bumps(scores, codes);
        }
    }
    ABILITY_CODES.map(|code| (code, scores.get(code)))
}

/// Player-facing messages for unmet multiclass ability prerequisites.
/// Empty if enforcement is off, there's only one class, or every class
/// meets at least one OR-alternative. Uses post-species/ASI `final_abilities`.
pub fn multiclass_prereq_violations(
    character: &Character,
    classes: &HashMap<String, Class>,
    species: Option<&Species>,
) -> Vec<String> {
    if !character.enforce_multiclass_prereqs || character.classes.len() < 2 {
        return Vec::new();
    }
    let scores = final_abilities(character, species);
    let get = |code: &str| scores.iter().find(|(c, _)| *c == code).map(|(_, v)| *v).unwrap_or(0);

    character
        .classes
        .iter()
        .filter_map(|entry| {
            let class = classes.get(&entry.class_id)?;
            if class.multiclass_ability_prerequisites.is_empty() {
                return None;
            }
            let met = class
                .multiclass_ability_prerequisites
                .iter()
                .any(|group| group.iter().all(|(ability, min)| get(ability) >= *min));
            if met {
                return None;
            }
            let alternatives = class
                .multiclass_ability_prerequisites
                .iter()
                .map(|group| {
                    group
                        .iter()
                        .map(|(ability, min)| format!("{} {min}", ability_label(ability)))
                        .collect::<Vec<_>>()
                        .join(" and ")
                })
                .collect::<Vec<_>>()
                .join(" or ");
            Some(format!("{} requires {alternatives}", class.name))
        })
        .collect()
}

pub fn hp_max(hit_die: u8, level: u8, con_mod: i8) -> i32 {
    let level = level.max(1) as i32;
    let per_level = (hit_die / 2 + 1) as i32;
    let total = hit_die as i32 + (level - 1) * per_level + level * con_mod as i32;
    total.max(level)
}

/// The average-method HP gained from one level after the first, for a given
/// hit die. Each level after 1 gains hit_die/2+1 (rounded up) plus Con mod,
/// floored at 1 per level (never 0 or negative).
fn average_hp_gain(hit_die: u8, con_mod: i8) -> i32 {
    let per_level = (hit_die / 2 + 1) as i32;
    (per_level + con_mod as i32).max(1)
}

/// HP max under whichever `HpMethod` the character actually chose, across
/// every class. Level 1 overall is always the *first* class's own hit die,
/// maxed (RAW — never rolled); every level after that, in any class, uses
/// that level's own class's hit die. `hit_dice` maps each `class_id` to its
/// hit die. `Rolled` uses `hp_rolls` (one stored roll per level after 1st,
/// in `classes` order) instead of the average formula; `Manual` falls back
/// to the average total if `hp_manual` is unset, so an incomplete Manual
/// entry never renders as 0/blank.
pub fn resolved_hp_max_multiclass(character: &Character, hit_dice: &HashMap<String, u8>, con_mod: i8) -> i32 {
    let Some(first) = character.classes.first() else { return 0 };
    let total = total_level(&character.classes) as i32;
    let level_one = (hit_dice.get(&first.class_id).copied().unwrap_or(1) as i32 + con_mod as i32).max(1);

    let average_total = || {
        let rest: i32 = character
            .classes
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                let levels = if index == 0 { entry.level.saturating_sub(1) } else { entry.level };
                let die = hit_dice.get(&entry.class_id).copied().unwrap_or(0);
                levels as i32 * average_hp_gain(die, con_mod)
            })
            .sum();
        (level_one + rest).max(total)
    };

    match character.hp_method {
        HpMethod::Average => average_total(),
        HpMethod::Rolled => {
            let rest: i32 = character.hp_rolls.iter().map(|&roll| roll as i32 + con_mod as i32).sum();
            (level_one + rest).max(total)
        }
        HpMethod::Manual => character.hp_manual.unwrap_or_else(average_total),
    }
}

pub fn asi_levels(features: &[ClassFeature]) -> Vec<u8> {
    let mut levels: Vec<u8> = features
        .iter()
        .filter(|feature| feature.name == "Ability Score Improvement")
        .map(|feature| feature.level)
        .collect();
    levels.sort_unstable();
    levels.dedup();
    levels
}

/// Total ASI-slot count across every class a multiclass character has: each
/// class's own `asi_levels` gated by that *class's own* level, not the
/// character's total — a level-5 Fighter/3 Wizard gets Fighter's ASI once
/// Fighter itself reaches level 4, independent of when Wizard levels were
/// taken (this app has no level-up history, only current per-class totals).
pub fn multiclass_asi_slots(classes: &[ClassLevel], features_by_class: &HashMap<String, Vec<ClassFeature>>) -> usize {
    classes
        .iter()
        .map(|entry| {
            features_by_class
                .get(&entry.class_id)
                .map(|features| asi_levels(features).iter().filter(|&&level| level <= entry.level).count())
                .unwrap_or(0)
        })
        .sum()
}

// SRD spell slot tables, indexed [level - 1][spell level - 1]. Half-casters
// only ever fill columns 0-4 (they cap at 5th-level spells).
const FULL_CASTER_SLOTS: [[u8; 9]; 20] = [
    [2, 0, 0, 0, 0, 0, 0, 0, 0],
    [3, 0, 0, 0, 0, 0, 0, 0, 0],
    [4, 2, 0, 0, 0, 0, 0, 0, 0],
    [4, 3, 0, 0, 0, 0, 0, 0, 0],
    [4, 3, 2, 0, 0, 0, 0, 0, 0],
    [4, 3, 3, 0, 0, 0, 0, 0, 0],
    [4, 3, 3, 1, 0, 0, 0, 0, 0],
    [4, 3, 3, 2, 0, 0, 0, 0, 0],
    [4, 3, 3, 3, 1, 0, 0, 0, 0],
    [4, 3, 3, 3, 2, 0, 0, 0, 0],
    [4, 3, 3, 3, 2, 1, 0, 0, 0],
    [4, 3, 3, 3, 2, 1, 0, 0, 0],
    [4, 3, 3, 3, 2, 1, 1, 0, 0],
    [4, 3, 3, 3, 2, 1, 1, 0, 0],
    [4, 3, 3, 3, 2, 1, 1, 1, 0],
    [4, 3, 3, 3, 2, 1, 1, 1, 0],
    [4, 3, 3, 3, 2, 1, 1, 1, 1],
    [4, 3, 3, 3, 3, 1, 1, 1, 1],
    [4, 3, 3, 3, 3, 2, 1, 1, 1],
    [4, 3, 3, 3, 3, 2, 2, 1, 1],
];
const HALF_CASTER_SLOTS: [[u8; 9]; 20] = [
    [0, 0, 0, 0, 0, 0, 0, 0, 0],
    [2, 0, 0, 0, 0, 0, 0, 0, 0],
    [3, 0, 0, 0, 0, 0, 0, 0, 0],
    [3, 0, 0, 0, 0, 0, 0, 0, 0],
    [4, 2, 0, 0, 0, 0, 0, 0, 0],
    [4, 2, 0, 0, 0, 0, 0, 0, 0],
    [4, 3, 0, 0, 0, 0, 0, 0, 0],
    [4, 3, 0, 0, 0, 0, 0, 0, 0],
    [4, 3, 2, 0, 0, 0, 0, 0, 0],
    [4, 3, 2, 0, 0, 0, 0, 0, 0],
    [4, 3, 3, 0, 0, 0, 0, 0, 0],
    [4, 3, 3, 0, 0, 0, 0, 0, 0],
    [4, 3, 3, 1, 0, 0, 0, 0, 0],
    [4, 3, 3, 1, 0, 0, 0, 0, 0],
    [4, 3, 3, 2, 0, 0, 0, 0, 0],
    [4, 3, 3, 2, 0, 0, 0, 0, 0],
    [4, 3, 3, 3, 1, 0, 0, 0, 0],
    [4, 3, 3, 3, 1, 0, 0, 0, 0],
    [4, 3, 3, 3, 2, 0, 0, 0, 0],
    [4, 3, 3, 3, 2, 0, 0, 0, 0],
];

/// One-third caster (Eldritch Knight/Arcane Trickster) slot table — not
/// derivable from `FULL_CASTER_SLOTS` by dividing level by 3 (the two
/// diverge at levels 19-20), so it's its own printed table, capped at
/// 4th-level spells.
const THIRD_CASTER_SLOTS: [[u8; 9]; 20] = [
    [0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0],
    [2, 0, 0, 0, 0, 0, 0, 0, 0],
    [3, 0, 0, 0, 0, 0, 0, 0, 0],
    [3, 0, 0, 0, 0, 0, 0, 0, 0],
    [3, 0, 0, 0, 0, 0, 0, 0, 0],
    [4, 2, 0, 0, 0, 0, 0, 0, 0],
    [4, 2, 0, 0, 0, 0, 0, 0, 0],
    [4, 2, 0, 0, 0, 0, 0, 0, 0],
    [4, 3, 0, 0, 0, 0, 0, 0, 0],
    [4, 3, 0, 0, 0, 0, 0, 0, 0],
    [4, 3, 0, 0, 0, 0, 0, 0, 0],
    [4, 3, 2, 0, 0, 0, 0, 0, 0],
    [4, 3, 2, 0, 0, 0, 0, 0, 0],
    [4, 3, 2, 0, 0, 0, 0, 0, 0],
    [4, 3, 3, 0, 0, 0, 0, 0, 0],
    [4, 3, 3, 0, 0, 0, 0, 0, 0],
    [4, 3, 3, 0, 0, 0, 0, 0, 0],
    [4, 3, 3, 1, 0, 0, 0, 0, 0],
    [4, 3, 3, 1, 0, 0, 0, 0, 0],
];

/// Pact Magic (Warlock): `(slot_level, slot_count)` per character level —
/// unlike full/half/third casters, all of a Warlock's slots share one spell
/// level rather than being spread across several.
const PACT_SLOTS: [(u8, u8); 20] = [
    (1, 1),
    (1, 2),
    (2, 2),
    (2, 2),
    (3, 2),
    (3, 2),
    (4, 2),
    (4, 2),
    (5, 2),
    (5, 2),
    (5, 3),
    (5, 3),
    (5, 3),
    (5, 3),
    (5, 3),
    (5, 3),
    (5, 4),
    (5, 4),
    (5, 4),
    (5, 4),
];

/// Spell slots by spell level (index 0 = 1st-level slots) for the given
/// normalized `caster_progression` ("full"/"half"/"third"/"pact"/"artificer";
/// anything else, including absent, isn't modeled and gets zero slots).
pub fn spell_slots_for_level(progression: &str, level: u8) -> [u8; 9] {
    let index = level.saturating_sub(1).min(19) as usize;
    match progression {
        "full" => FULL_CASTER_SLOTS[index],
        "half" => HALF_CASTER_SLOTS[index],
        "third" => THIRD_CASTER_SLOTS[index],
        // Artificer's level-1 row borrows the half-caster's level-2 row,
        // then tracks the half-caster table exactly from level 2 on.
        "artificer" => HALF_CASTER_SLOTS[(level.max(2) as usize - 1).min(19)],
        "pact" => {
            let (slot_level, slot_count) = PACT_SLOTS[index];
            let mut slots = [0; 9];
            slots[slot_level as usize - 1] = slot_count;
            slots
        }
        _ => [0; 9],
    }
}

/// Highest spell level with a nonzero slot, for pool-filtering purposes (0 if
/// the class has no slots at this level/progression).
pub fn max_castable_spell_level(progression: &str, level: u8) -> u8 {
    spell_slots_for_level(progression, level)
        .iter()
        .rposition(|&count| count > 0)
        .map_or(0, |index| index as u8 + 1)
}

/// Sums 5e's multiclass "caster level" across every full/half/third-caster
/// class a character has (full counts its own level, half/artificer count
/// level/2 rounded down, third counts level/3 rounded down); Pact Magic
/// (Warlock) is never summed into this — see `multiclass_pact_slots`.
/// `progressions` maps each `class_id` to that class's `caster_progression`.
pub fn multiclass_caster_level(classes: &[ClassLevel], progressions: &HashMap<String, Option<String>>) -> u8 {
    classes
        .iter()
        .map(|entry| match progressions.get(&entry.class_id).and_then(|p| p.as_deref()) {
            Some("full") => entry.level,
            Some("half") | Some("artificer") => entry.level / 2,
            Some("third") => entry.level / 3,
            _ => 0,
        })
        .sum()
}

/// The shared spell-slot pool. 5e's combined-caster-level formula only
/// applies when a character actually has 2+ *distinct* spellcasting classes
/// (Pact Magic aside) — a solo half/third caster (Paladin, Ranger, Eldritch
/// Knight, ...) uses their own class's own table at their own level, which
/// the `floor(level/2)`-into-the-full-table formula does NOT reproduce
/// (their printed tables are front-loaded differently). Only a full caster
/// happens to be formula-identity-preserving solo, since its contribution
/// is just its own level looked up in the same table it already uses.
pub fn multiclass_spell_slots(classes: &[ClassLevel], progressions: &HashMap<String, Option<String>>) -> [u8; 9] {
    let casters: Vec<&ClassLevel> = classes
        .iter()
        .filter(|entry| {
            matches!(
                progressions.get(&entry.class_id).and_then(|p| p.as_deref()),
                Some("full") | Some("half") | Some("third") | Some("artificer")
            )
        })
        .collect();
    match casters.as_slice() {
        [] => [0; 9],
        [only] => {
            let progression = progressions.get(&only.class_id).and_then(|p| p.as_deref()).unwrap_or("");
            spell_slots_for_level(progression, only.level)
        }
        _ => spell_slots_for_level("full", multiclass_caster_level(classes, progressions)),
    }
}

/// Pact Magic slots stay entirely separate from the shared multiclass pool,
/// summed per entry across any classes with `"pact"` progression (in
/// practice just Warlock, but this doesn't hardcode that assumption).
pub fn multiclass_pact_slots(classes: &[ClassLevel], progressions: &HashMap<String, Option<String>>) -> [u8; 9] {
    let mut slots = [0u8; 9];
    for entry in classes {
        if progressions.get(&entry.class_id).and_then(|p| p.as_deref()) == Some("pact") {
            let pact = spell_slots_for_level("pact", entry.level);
            for (total, gained) in slots.iter_mut().zip(pact.iter()) {
                *total = total.saturating_add(*gained);
            }
        }
    }
    slots
}

/// What actually governs a character's spellcasting: the class's own fields
/// if it's a caster, otherwise the selected subclass's (for subclass-granted
/// casters like Eldritch Knight/Arcane Trickster, whose base class is
/// correctly a non-caster). No class both casts itself *and* grants casting
/// via a subclass, so falling back only when the class has no
/// `caster_progression` is safe — a non-casting subclass of a casting class
/// never overrides its class's own fields.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SpellcastingProfile {
    pub caster_progression: Option<String>,
    pub spellcasting_ability: Option<String>,
    pub spells_known_progression: Option<Vec<u8>>,
    pub cantrips_known_progression: Option<Vec<u8>>,
}

pub fn effective_spellcasting(class: &Class, subclass: Option<&Subclass>) -> SpellcastingProfile {
    if class.caster_progression.is_some() {
        return SpellcastingProfile {
            caster_progression: class.caster_progression.clone(),
            spellcasting_ability: class.spellcasting_ability.clone(),
            spells_known_progression: class.spells_known_progression.clone(),
            cantrips_known_progression: class.cantrips_known_progression.clone(),
        };
    }
    match subclass.filter(|s| s.caster_progression.is_some()) {
        Some(subclass) => SpellcastingProfile {
            caster_progression: subclass.caster_progression.clone(),
            spellcasting_ability: subclass.spellcasting_ability.clone(),
            spells_known_progression: subclass.spells_known_progression.clone(),
            cantrips_known_progression: subclass.cantrips_known_progression.clone(),
        },
        None => SpellcastingProfile::default(),
    }
}

/// Cantrips known at a level; `None` progression (e.g. half-casters) means 0.
pub fn cantrips_known_count(profile: &SpellcastingProfile, level: u8) -> usize {
    profile
        .cantrips_known_progression
        .as_ref()
        .and_then(|progression| progression.get(level.saturating_sub(1) as usize))
        .copied()
        .unwrap_or(0) as usize
}

/// Spells known/prepared at a level. A fixed `spells_known_progression`
/// table (Sorcerer/Bard/Warlock-style known casters) takes precedence;
/// otherwise, a caster with a `spellcasting_ability` prepares
/// `level_component + mod` (min 1), where `level_component` is `level` for
/// full casters and `level / 2` for half casters and Artificer (same
/// formula, per the fixture's `preparedSpells` text) — the actual rules
/// difference between e.g. Wizard and Paladin.
pub fn spells_known_count(profile: &SpellcastingProfile, level: u8, ability_score: u8) -> usize {
    if let Some(progression) = &profile.spells_known_progression {
        return progression.get(level.saturating_sub(1) as usize).copied().unwrap_or(0) as usize;
    }
    if profile.spellcasting_ability.is_none() {
        return 0;
    }
    let level_component = match profile.caster_progression.as_deref() {
        Some("full") => level as i32,
        Some("half") | Some("artificer") => (level / 2) as i32,
        _ => return 0,
    };
    (level_component + ability_modifier(ability_score) as i32).max(1) as usize
}

pub fn spell_save_dc(prof_bonus: u8, ability_mod: i8) -> u8 {
    (8 + prof_bonus as i8 + ability_mod).max(0) as u8
}

pub fn spell_attack_bonus(prof_bonus: u8, ability_mod: i8) -> i8 {
    prof_bonus as i8 + ability_mod
}

/// One class's spellcasting picture within a multiclass character: the
/// *slot pool* is shared (`multiclass_spell_slots`), but how many
/// spells/cantrips this class knows or prepares is still computed against
/// its own level and its own spellcasting ability score.
#[derive(Debug, Clone, PartialEq)]
pub struct MulticlassSpellProfile {
    pub class_id: String,
    pub profile: SpellcastingProfile,
    pub known_spells: usize,
    pub known_cantrips: usize,
}

/// Per-class known/prepared spell and cantrip counts for every caster class
/// a multiclass character has. Non-caster classes (and classes missing from
/// `class_lookup`) are skipped. `final_abilities` is the character's final
/// ability scores (post species/ASI), as returned by `final_abilities`.
pub fn multiclass_spell_profiles(
    classes: &[ClassLevel],
    class_lookup: &HashMap<String, (Class, Option<Subclass>)>,
    final_abilities: &[(&str, u8); 6],
) -> Vec<MulticlassSpellProfile> {
    classes
        .iter()
        .filter_map(|entry| {
            let (class, subclass) = class_lookup.get(&entry.class_id)?;
            let profile = effective_spellcasting(class, subclass.as_ref());
            profile.caster_progression.as_ref()?;
            let ability_score = profile
                .spellcasting_ability
                .as_deref()
                .and_then(|code| final_abilities.iter().find(|(c, _)| *c == code))
                .map(|(_, score)| *score)
                .unwrap_or(10);
            Some(MulticlassSpellProfile {
                class_id: entry.class_id.clone(),
                known_spells: spells_known_count(&profile, entry.level, ability_score),
                known_cantrips: cantrips_known_count(&profile, entry.level),
                profile,
            })
        })
        .collect()
}

/// How many picks of `feature_type` a character can make at `level` — the
/// **sum** of every matching-type progression across the class and its
/// active subclass, not a class-or-subclass fallback like spellcasting.
/// This is what makes a Champion's second Fighting Style (a subclass-level
/// grant on top of Fighter's own class-level one) add up correctly with no
/// special-casing.
pub fn optional_feature_quota(
    class: &Class,
    subclass: Option<&Subclass>,
    feature_type: FeatureType,
    level: u8,
) -> usize {
    let idx = level.saturating_sub(1) as usize;
    let sum = |progressions: &[crate::models::class::OptionalFeatureProgression]| -> usize {
        progressions
            .iter()
            .filter(|p| p.feature_type == feature_type)
            .map(|p| p.known.get(idx).copied().unwrap_or(0) as usize)
            .sum()
    };
    sum(&class.optional_feature_progressions)
        + subclass.map(|s| sum(&s.optional_feature_progressions)).unwrap_or(0)
}

/// Every `FeatureType` with a nonzero quota at `level`, across the class and
/// its active subclass — the wizard's section list, and the single `types`
/// filter for fetching every relevant pool in one call.
pub fn active_optional_feature_types(
    class: &Class,
    subclass: Option<&Subclass>,
    level: u8,
) -> Vec<FeatureType> {
    let mut types: Vec<FeatureType> = class
        .optional_feature_progressions
        .iter()
        .chain(subclass.into_iter().flat_map(|s| s.optional_feature_progressions.iter()))
        .map(|p| p.feature_type)
        .collect();
    types.sort_by_key(FeatureType::as_str);
    types.dedup();
    types.into_iter().filter(|&t| optional_feature_quota(class, subclass, t, level) > 0).collect()
}

/// Sums `optional_feature_quota` across every class (+ its own active
/// subclass) a multiclass character has — each entry supplies its own
/// class, active subclass, and its *own* level.
pub fn multiclass_optional_feature_quota(
    entries: &[(&Class, Option<&Subclass>, u8)],
    feature_type: FeatureType,
) -> usize {
    entries
        .iter()
        .map(|(class, subclass, level)| optional_feature_quota(class, *subclass, feature_type, *level))
        .sum()
}

/// Every `FeatureType` with a nonzero quota across any class (+ active
/// subclass) a multiclass character has.
pub fn multiclass_active_optional_feature_types(entries: &[(&Class, Option<&Subclass>, u8)]) -> Vec<FeatureType> {
    let mut types: Vec<FeatureType> = entries
        .iter()
        .flat_map(|(class, subclass, level)| active_optional_feature_types(class, *subclass, *level))
        .collect();
    types.sort_by_key(FeatureType::as_str);
    types.dedup();
    types
}

pub fn subclass_unlock_level(detail: &ClassDetail) -> Option<u8> {
    detail
        .subclasses
        .iter()
        .flat_map(|subclass| subclass.features.iter().map(|feature| feature.level))
        .min()
}

/// Resolves the character's actually-active subclass: `None` unless `level`
/// has reached `subclass_unlock_level`, even if `subclass_id` names a real
/// subclass — a character (or a direct POST to `save_character`, bypassing
/// the wizard's own client-side gating) can carry a subclass_id ahead of the
/// level that actually unlocks it, and that must not grant its spellcasting
/// (or any other subclass-gated benefit) early.
pub fn active_subclass<'a>(
    class_detail: &'a ClassDetail,
    subclass_id: Option<&str>,
    level: u8,
) -> Option<&'a Subclass> {
    let unlocked = subclass_unlock_level(class_detail).is_some_and(|unlock| unlock <= level);
    unlocked.then(|| class_detail.subclass(subclass_id)).flatten()
}

/// One required skill choice-pool slot, labelled with its granting class/background.
#[derive(Debug, Clone, PartialEq)]
pub struct SkillChoicePool {
    pub source: String,
    pub options: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SkillSlots {
    /// Skills granted outright by class + background, deduped.
    pub fixed: Vec<String>,
    /// One entry per required pick-slot: the grant's own option list, or the
    /// full `SKILLS` list if that grant's own list can't satisfy its count
    /// once already-fixed skills are excluded (see `skill_slots`).
    pub choice_pools: Vec<SkillChoicePool>,
}

/// Merges class + background skill grants into the fixed set and the
/// required choice slots. A `Choose`/`Any` grant's options are its own
/// specific list where possible (rules-accurate, restricted choice); if
/// that list can't supply enough options *not already fixed-granted* to
/// satisfy its count (e.g. a class offers "choose 2 from Athletics/
/// Intimidation/Survival" but the background already fixed-grants two of
/// those three), every slot for that grant falls back to the full skill
/// list instead of shrinking the required count — this app doesn't track
/// tools/languages yet, so a proper 5e "replacement proficiency" isn't
/// feasible (see TDD.md, Phase 6 notes).
/// Combines skill-proficiency grants across every class a multiclass
/// character has: only the first (primary) class's full `proficiencies`
/// apply — 5e only grants full starting proficiencies to a character's
/// first class — every class taken afterward only grants its smaller
/// `multiclass_proficiencies`. Feed the result into `skill_slots` as its
/// `class_skills` argument, same as a single class always has been.
pub fn multiclass_class_skill_grants(
    classes: &[ClassLevel],
    class_lookup: &HashMap<String, Class>,
) -> Vec<(String, SkillGrant)> {
    let mut combined = Vec::new();
    for (index, entry) in classes.iter().enumerate() {
        let Some(class) = class_lookup.get(&entry.class_id) else { continue };
        let profs = if index == 0 { &class.proficiencies } else { &class.multiclass_proficiencies };
        let source = if index == 0 { class.name.clone() } else { format!("{} (multiclass)", class.name) };
        combined.extend(profs.skills.iter().cloned().map(|grant| (source.clone(), grant)));
    }
    combined
}

pub fn skill_slots(
    class_skills: &[(String, SkillGrant)],
    background_skills: &[(String, SkillGrant)],
) -> SkillSlots {
    let mut fixed = Vec::new();
    for (_, grant) in class_skills.iter().chain(background_skills.iter()) {
        if let SkillGrant::Fixed { skills } = grant {
            fixed.extend(skills.iter().cloned());
        }
    }
    fixed.sort();
    fixed.dedup();

    let all_skills: Vec<String> = SKILLS.iter().map(|(name, _)| name.to_string()).collect();

    let mut choice_pools = Vec::new();
    for (source, grant) in class_skills.iter().chain(background_skills.iter()) {
        match grant {
            SkillGrant::Fixed { .. } => {}
            SkillGrant::Choose { count, from } => {
                let available = from.iter().filter(|skill| !fixed.contains(skill)).count();
                let pool = if available >= *count as usize { from.clone() } else { all_skills.clone() };
                for _ in 0..*count {
                    choice_pools.push(SkillChoicePool { source: source.clone(), options: pool.clone() });
                }
            }
            SkillGrant::Any { count } => {
                for _ in 0..*count {
                    choice_pools.push(SkillChoicePool { source: source.clone(), options: all_skills.clone() });
                }
            }
        }
    }

    SkillSlots { fixed, choice_pools }
}

/// One required language choice-pool slot, labelled with its granting
/// background/species — mirrors `SkillChoicePool`.
#[derive(Debug, Clone, PartialEq)]
pub struct LanguageChoicePool {
    pub source: String,
    pub options: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LanguageSlots {
    /// Languages granted outright by background + species, deduped.
    pub fixed: Vec<String>,
    /// One entry per required pick-slot: the grant's own option list, or
    /// `all_language_names` if that list can't satisfy its count once
    /// already-fixed languages are excluded (see `language_slots`).
    pub choice_pools: Vec<LanguageChoicePool>,
}

/// Merges background + species language grants into the fixed set and the
/// required choice slots, same fallback logic as `skill_slots` — a `Choose`
/// grant's own `from` list is used where possible; if it can't supply enough
/// options not already fixed-granted to satisfy its count, every slot for
/// that grant falls back to `all_language_names` (the full DB-backed
/// `Language` list) instead of shrinking the required count.
pub fn language_slots(
    background_languages: &[(String, LanguageGrant)],
    species_languages: &[(String, LanguageGrant)],
    all_language_names: &[String],
) -> LanguageSlots {
    let mut fixed = Vec::new();
    for (_, grant) in background_languages.iter().chain(species_languages.iter()) {
        if let LanguageGrant::Fixed { languages } = grant {
            fixed.extend(languages.iter().cloned());
        }
    }
    fixed.sort();
    fixed.dedup();

    let mut choice_pools = Vec::new();
    for (source, grant) in background_languages.iter().chain(species_languages.iter()) {
        match grant {
            LanguageGrant::Fixed { .. } => {}
            LanguageGrant::Choose { count, from } => {
                let available = from.iter().filter(|language| !contains_ignore_case(&fixed, language)).count();
                let pool = if available >= *count as usize {
                    from.clone()
                } else {
                    all_language_names.to_vec()
                };
                for _ in 0..*count {
                    choice_pools.push(LanguageChoicePool { source: source.clone(), options: pool.clone() });
                }
            }
            LanguageGrant::Any { count } => {
                for _ in 0..*count {
                    choice_pools.push(LanguageChoicePool {
                        source: source.clone(),
                        options: all_language_names.to_vec(),
                    });
                }
            }
        }
    }

    LanguageSlots { fixed, choice_pools }
}

/// One required tool choice-pool slot, labelled with its granting
/// class/background — mirrors `SkillChoicePool`.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolChoicePool {
    pub source: String,
    pub options: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolSlots {
    /// Tools granted outright by class + background, deduped.
    pub fixed: Vec<String>,
    /// One entry per required pick-slot: the grant's own option list (with
    /// any `ToolOption::Category`/`AnyCategory` entries expanded against
    /// `category_members`), or every category's members if that list can't
    /// satisfy its count once already-fixed tools are excluded.
    pub choice_pools: Vec<ToolChoicePool>,
}

/// Combines tool-proficiency grants across every class a multiclass
/// character has, same primary/multiclass split as `multiclass_class_skill_grants`.
pub fn multiclass_class_tool_grants(
    classes: &[ClassLevel],
    class_lookup: &HashMap<String, Class>,
) -> Vec<(String, ToolGrant)> {
    let mut combined = Vec::new();
    for (index, entry) in classes.iter().enumerate() {
        let Some(class) = class_lookup.get(&entry.class_id) else { continue };
        let profs = if index == 0 { &class.proficiencies } else { &class.multiclass_proficiencies };
        let source = if index == 0 { class.name.clone() } else { format!("{} (multiclass)", class.name) };
        combined.extend(profs.tools.iter().cloned().map(|grant| (source.clone(), grant)));
    }
    combined
}

fn expand_tool_option(option: &ToolOption, category_members: &HashMap<ToolCategory, Vec<String>>) -> Vec<String> {
    match option {
        ToolOption::Named(name) => vec![name.clone()],
        ToolOption::Category(category) => category_members.get(category).cloned().unwrap_or_default(),
    }
}

/// Merges class + background tool grants into the fixed set and the
/// required choice slots, same fallback logic as `skill_slots`. Category
/// tokens (`ToolOption::Category`/`AnyCategory`) are expanded against
/// `category_members` — real tool item names grouped by `ToolCategory`,
/// resolved from the `items` table's `item_type_code` column rather than a
/// hardcoded member list (see `models::proficiency::ToolCategory`).
pub fn tool_slots(
    class_tools: &[(String, ToolGrant)],
    background_tools: &[(String, ToolGrant)],
    category_members: &HashMap<ToolCategory, Vec<String>>,
) -> ToolSlots {
    let mut fixed = Vec::new();
    for (_, grant) in class_tools.iter().chain(background_tools.iter()) {
        if let ToolGrant::Fixed { tools } = grant {
            fixed.extend(tools.iter().cloned());
        }
    }
    fixed.sort();
    fixed.dedup();

    let mut all_tools: Vec<String> = category_members.values().flatten().cloned().collect();
    all_tools.sort();
    all_tools.dedup();

    let mut choice_pools = Vec::new();
    for (source, grant) in class_tools.iter().chain(background_tools.iter()) {
        match grant {
            ToolGrant::Fixed { .. } => {}
            ToolGrant::Choose { count, from } => {
                let expanded: Vec<String> =
                    from.iter().flat_map(|option| expand_tool_option(option, category_members)).collect();
                let available = expanded.iter().filter(|tool| !contains_ignore_case(&fixed, tool)).count();
                let pool = if available >= *count as usize { expanded } else { all_tools.clone() };
                for _ in 0..*count {
                    choice_pools.push(ToolChoicePool { source: source.clone(), options: pool.clone() });
                }
            }
            ToolGrant::Any { count } => {
                for _ in 0..*count {
                    choice_pools.push(ToolChoicePool { source: source.clone(), options: all_tools.clone() });
                }
            }
            ToolGrant::AnyCategory { count, category } => {
                let options = category_members.get(category).cloned().unwrap_or_default();
                for _ in 0..*count {
                    choice_pools.push(ToolChoicePool { source: source.clone(), options: options.clone() });
                }
            }
        }
    }

    ToolSlots { fixed, choice_pools }
}

/// Each "Expertise" class feature (Rogue 1 & 6, Bard 3 & 10 in real 5e)
/// grants exactly 2 expertise picks — matches the rules rather than parsing
/// the feature's prose, mirroring how `asi_levels` keys off an exact
/// feature-name match instead.
pub fn expertise_slots(features: &[ClassFeature], level: u8) -> usize {
    features.iter().filter(|feature| feature.name == "Expertise" && feature.level <= level).count() * 2
}

/// Sums `expertise_slots` across every class a multiclass character has,
/// each gated on that class's own level.
pub fn multiclass_expertise_slots(
    classes: &[ClassLevel],
    features_by_class: &HashMap<String, Vec<ClassFeature>>,
) -> usize {
    classes
        .iter()
        .map(|entry| {
            features_by_class
                .get(&entry.class_id)
                .map(|features| expertise_slots(features, entry.level))
                .unwrap_or(0)
        })
        .sum()
}

#[cfg(test)]
#[path = "character_tests.rs"]
mod tests;
