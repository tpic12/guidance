use crate::models::background::BackgroundQuery;
use crate::models::character::{
    active_optional_feature_types, active_subclass, asi_levels, cantrips_known_count, effective_spellcasting,
    final_abilities, language_slots, max_castable_spell_level, multiclass_active_optional_feature_types,
    multiclass_asi_slots, multiclass_class_skill_grants, multiclass_class_tool_grants, multiclass_expertise_slots,
    multiclass_prereq_violations, multiclass_spell_profiles, optional_feature_quota, point_buy_cost, skill_slots,
    spells_known_count, subclass_unlock_level, tool_slots, total_level, AbilityBonusSource, AbilityMethod,
    AbilityScores, AsiChoice, Character, ClassLevel, HpMethod, SpellcastingProfile, ABILITY_CODES,
    POINT_BUY_BUDGET,
};
use crate::models::class::{ability_label, Class, ClassDetail, Subclass};
use crate::models::feat::FeatQuery;
use crate::models::language::LanguageGrant;
use crate::models::optional_feature::{is_eligible, EligibilityContext, FeatureType, OptionalFeature, OptionalFeatureQuery};
use crate::models::proficiency::{contains_ignore_case, title_case, ToolCategory, ToolGrant};
use crate::models::skill::{skill_label, SkillGrant};
use crate::models::species::{is_free_ability_choice, AbilityBonusGrant, SpeciesQuery};
use crate::models::spell::Spell;
use crate::pages::backgrounds::{get_background, get_backgrounds};
use crate::pages::class_detail::get_class_detail;
use crate::pages::classes::get_classes;
use crate::pages::feats::get_feats;
use crate::pages::optional_features::get_optional_features;
use crate::pages::species::{get_species, get_species_by_id};
use crate::pages::spells::{get_class_spell_options, get_subclass_granted_spells, get_subclass_spell_choice_pools};
use leptos::prelude::*;
use leptos::server_fn::codec::Json;
use leptos_router::hooks::{use_navigate, use_params_map};
use std::collections::{HashMap, HashSet};

mod steps;
mod utils;

use steps::*;
use utils::attribute_multiclass_choices;

#[server]
pub async fn get_character(id: String) -> Result<Option<Character>, ServerFnError> {
    use sqlx::SqlitePool;
    use tower_sessions::Session;

    let pool = expect_context::<SqlitePool>();
    let session: Session = leptos_axum::extract().await?;
    let user = crate::auth::require_login(&pool, &session).await?;
    crate::db::get_character(&pool, &id, &user.id)
        .await
        .map_err(|err| ServerFnError::new(err.to_string()))
}

/// The full language reference list, for the languages step's choice-pool
/// fallback — languages are DB-backed (unlike skills' compile-time `SKILLS`
/// constant), so the wizard needs this to resolve `Any`/exhausted-`Choose`
/// pools to real names.
#[server]
pub async fn get_languages() -> Result<Vec<crate::models::language::Language>, ServerFnError> {
    use sqlx::SqlitePool;

    let pool = expect_context::<SqlitePool>();
    crate::db::list_languages(&pool).await.map_err(|err| ServerFnError::new(err.to_string()))
}

/// Real tool item names for each `ToolCategory`, resolved from the `items`
/// table's `item_type_code` rather than a hardcoded list — feeds the tools
/// step's category-scoped choice pools (`anyArtisansTool`, ...). Returned as
/// pairs rather than a map since `ToolCategory` isn't a string-keyable JSON
/// map key.
#[server]
pub async fn get_tool_category_members() -> Result<Vec<(ToolCategory, Vec<String>)>, ServerFnError> {
    use sqlx::SqlitePool;

    let pool = expect_context::<SqlitePool>();
    let mut members = Vec::new();
    for category in [ToolCategory::ArtisansTool, ToolCategory::GamingSet, ToolCategory::MusicalInstrument] {
        let items = crate::db::list_items_by_type_code(&pool, category.item_type_code())
            .await
            .map_err(|err| ServerFnError::new(err.to_string()))?;
        members.push((category, items.into_iter().map(|item| item.name).collect()));
    }
    Ok(members)
}

// input = Json: the default URL-encoded codec drops empty vecs and None
// entries, corrupting asi_choices on the round trip.
#[server(input = Json)]
pub async fn save_character(character: Character) -> Result<String, ServerFnError> {
    use sqlx::SqlitePool;
    use tower_sessions::Session;

    character.validate().map_err(ServerFnError::new)?;
    let pool = expect_context::<SqlitePool>();
    let session: Session = leptos_axum::extract().await?;
    let user = crate::auth::require_login(&pool, &session).await?;

    // The wizard only ever offers as many ASI slots (and skill/spell/feature
    // picks) as each class actually unlocks, but that's UI-only — re-derive
    // everything here, per class, so a direct POST can't persist more (or
    // mismatched) choices than this character's classes actually grant.
    let mut class_details: HashMap<String, ClassDetail> = HashMap::new();
    for entry in &character.classes {
        let detail = crate::db::get_class_detail(&pool, &entry.class_id)
            .await
            .map_err(|err| ServerFnError::new(err.to_string()))?
            .ok_or_else(|| ServerFnError::new("Unknown class"))?;
        class_details.insert(entry.class_id.clone(), detail);
    }

    // Mirrors the wizard's client-side `subclass_required` gate: once a
    // class's own subclass choice has unlocked (at that class's own level),
    // a direct POST shouldn't be able to skip picking one for it.
    let mut resolved: Vec<(&ClassLevel, &ClassDetail, Option<&Subclass>)> = Vec::new();
    for entry in &character.classes {
        let detail = &class_details[&entry.class_id];
        if subclass_unlock_level(detail).is_some_and(|unlock| unlock <= entry.level)
            && active_subclass(detail, entry.subclass_id.as_deref(), entry.level).is_none()
        {
            return Err(ServerFnError::new(format!(
                "A subclass must be selected for {} at this level",
                detail.class.name
            )));
        }
        resolved.push((entry, detail, active_subclass(detail, entry.subclass_id.as_deref(), entry.level)));
    }

    // Character::validate() can only check hp_rolls' length/nonzero-ness —
    // it has no access to any class's hit die, so each roll's upper bound is
    // re-checked here against that roll's own level's own class.
    if character.hp_method == HpMethod::Rolled {
        let mut rest_class_ids: Vec<&str> = Vec::new();
        for (index, entry) in character.classes.iter().enumerate() {
            let levels = if index == 0 { entry.level.saturating_sub(1) } else { entry.level };
            rest_class_ids.extend(std::iter::repeat(entry.class_id.as_str()).take(levels as usize));
        }
        for (&roll, class_id) in character.hp_rolls.iter().zip(rest_class_ids.iter()) {
            let hit_die = class_details[*class_id].class.hit_die;
            if roll < 1 || roll > hit_die {
                return Err(ServerFnError::new(format!(
                    "Rolled HP values must be between 1 and {hit_die} (that level's class hit die)"
                )));
            }
        }
    }

    let features_by_class: HashMap<String, Vec<crate::models::class::ClassFeature>> =
        class_details.iter().map(|(id, detail)| (id.clone(), detail.features.clone())).collect();
    let unlocked_asi_slots = multiclass_asi_slots(&character.classes, &features_by_class);
    if character.asi_choices.len() != unlocked_asi_slots {
        return Err(ServerFnError::new(
            "Ability score improvements don't match what's unlocked across this character's classes",
        ));
    }

    // Same reasoning as the ASI check above: re-derive the skill/expertise
    // slots from the DB-fetched classes+background so a direct POST can't
    // persist choices the wizard's own UI wouldn't have offered. Only the
    // first (primary) class grants its full proficiencies — every class
    // taken afterward only grants its smaller `multiclass_proficiencies`,
    // per 5e's multiclassing rule.
    let background = crate::db::get_background(&pool, &character.background_id)
        .await
        .map_err(|err| ServerFnError::new(err.to_string()))?
        .ok_or_else(|| ServerFnError::new("Unknown background"))?;
    let species = crate::db::get_species(&pool, &character.species_id)
        .await
        .map_err(|err| ServerFnError::new(err.to_string()))?
        .ok_or_else(|| ServerFnError::new("Unknown species"))?;
    let class_lookup: HashMap<String, Class> =
        class_details.iter().map(|(id, detail)| (id.clone(), detail.class.clone())).collect();
    let combined_class_skills = multiclass_class_skill_grants(&character.classes, &class_lookup);
    let background_skills: Vec<(String, SkillGrant)> =
        background.skills.iter().cloned().map(|grant| (background.name.clone(), grant)).collect();
    // A choice matching a fixed grant is allowed — it's a wasted pick, not
    // an invalid one; `Character::validate()` already rejected duplicates
    // within skill_choices itself, so picking the same already-fixed skill
    // twice across slots is caught there, not here.
    let slots = skill_slots(&combined_class_skills, &background_skills);
    let skill_choices_valid = character.skill_choices.len() == slots.choice_pools.len()
        && character
            .skill_choices
            .iter()
            .zip(&slots.choice_pools)
            .all(|(choice, pool)| pool.options.contains(choice));
    if !skill_choices_valid {
        return Err(ServerFnError::new(
            "Skill choices don't match what's unlocked for these classes and background",
        ));
    }

    let proficient: HashSet<&str> = slots
        .fixed
        .iter()
        .map(String::as_str)
        .chain(character.skill_choices.iter().map(String::as_str))
        .chain(character.custom_skill_proficiencies.iter().map(String::as_str))
        .collect();
    let needed_expertise = multiclass_expertise_slots(&character.classes, &features_by_class);
    let expertise_valid = character.expertise_choices.len() == needed_expertise
        && character.expertise_choices.iter().all(|skill| proficient.contains(skill.as_str()));
    if !expertise_valid {
        return Err(ServerFnError::new(
            "Expertise choices don't match what's unlocked, or name a skill you aren't proficient in",
        ));
    }

    // Same reasoning again, for languages and tools: re-derive both slot
    // sets from the DB-fetched classes+background+species so a direct POST
    // can't persist choices the wizard's own UI wouldn't have offered.
    let combined_class_tools = multiclass_class_tool_grants(&character.classes, &class_lookup);
    let background_tools: Vec<(String, ToolGrant)> =
        background.tools.iter().cloned().map(|grant| (background.name.clone(), grant)).collect();
    let mut category_members = HashMap::new();
    for category in [ToolCategory::ArtisansTool, ToolCategory::GamingSet, ToolCategory::MusicalInstrument] {
        let items = crate::db::list_items_by_type_code(&pool, category.item_type_code())
            .await
            .map_err(|err| ServerFnError::new(err.to_string()))?;
        category_members.insert(category, items.into_iter().map(|item| item.name).collect());
    }
    let tool_pick_slots = tool_slots(&combined_class_tools, &background_tools, &category_members);
    let tool_choices_valid = character.tool_choices.len() == tool_pick_slots.choice_pools.len()
        && character
            .tool_choices
            .iter()
            .zip(&tool_pick_slots.choice_pools)
            .all(|(choice, pool)| pool.options.contains(choice));
    if !tool_choices_valid {
        return Err(ServerFnError::new(
            "Tool choices don't match what's unlocked for these classes and background",
        ));
    }
    let custom_tool_proficiencies_valid = character
        .custom_tool_proficiencies
        .iter()
        .all(|tool| category_members.values().any(|items| contains_ignore_case(items, tool)));
    if !custom_tool_proficiencies_valid {
        return Err(ServerFnError::new("Custom tool proficiency isn't a recognized tool"));
    }

    let background_languages: Vec<(String, LanguageGrant)> =
        background.languages.iter().cloned().map(|grant| (background.name.clone(), grant)).collect();
    let species_languages: Vec<(String, LanguageGrant)> =
        species.languages.iter().cloned().map(|grant| (species.name.clone(), grant)).collect();
    let all_languages: Vec<String> =
        crate::db::list_languages(&pool).await.map_err(|err| ServerFnError::new(err.to_string()))?
            .into_iter()
            .map(|language| language.name)
            .collect();
    let language_pick_slots = language_slots(&background_languages, &species_languages, &all_languages);
    let language_choices_valid = character.language_choices.len() == language_pick_slots.choice_pools.len()
        && character
            .language_choices
            .iter()
            .zip(&language_pick_slots.choice_pools)
            .all(|(choice, pool)| pool.options.contains(choice));
    if !language_choices_valid {
        return Err(ServerFnError::new(
            "Language choices don't match what's unlocked for this background and species",
        ));
    }
    let custom_language_proficiencies_valid = character
        .custom_language_proficiencies
        .iter()
        .all(|language| contains_ignore_case(&all_languages, language));
    if !custom_language_proficiencies_valid {
        return Err(ServerFnError::new("Custom language proficiency isn't a recognized language"));
    }

    // Same reasoning again: re-derive the species' ability-bonus grant so a
    // direct POST can't persist a `species_ability_choices` pick the
    // wizard's own UI wouldn't have offered (wrong count, wrong ability, or
    // any pick at all for a species with no Choose grant) — or, in Custom
    // mode, a bonus that doesn't match the "exactly two valid abilities"
    // shape the wizard's Custom toggle offers.
    let ability_bonus_valid = match character.ability_bonus_source {
        AbilityBonusSource::Species => {
            if !character.custom_ability_bonus_choices.is_empty() {
                false
            } else {
                let species_choose_grant = species.ability_bonuses.iter().find_map(|grant| match grant {
                    AbilityBonusGrant::Choose { count, from, .. } => Some((*count, from.clone())),
                    AbilityBonusGrant::Fixed { .. } => None,
                });
                match &species_choose_grant {
                    Some((count, from)) => {
                        character.species_ability_choices.len() == *count as usize
                            && character.species_ability_choices.iter().all(|code| {
                                ABILITY_CODES.contains(&code.as_str())
                                    && (is_free_ability_choice(from) || from.contains(code))
                            })
                    }
                    None => character.species_ability_choices.is_empty(),
                }
            }
        }
        AbilityBonusSource::Custom => {
            character.species_ability_choices.is_empty()
                && character.custom_ability_bonus_choices.len() == 2
                && character
                    .custom_ability_bonus_choices
                    .iter()
                    .all(|code| ABILITY_CODES.contains(&code.as_str()))
                && character.custom_ability_bonus_choices[0] != character.custom_ability_bonus_choices[1]
        }
    };
    if !ability_bonus_valid {
        return Err(ServerFnError::new("Species ability choices don't match what this species offers"));
    }
    let finals = final_abilities(&character, Some(&species));

    // Same reasoning again, for spells: re-derive each class's own allowed
    // pool and required counts (its own subclass, its own level, its own
    // spellcasting ability score) from the DB-fetched classes so a direct
    // POST can't persist picks the wizard's own UI wouldn't have offered.
    // The *slot pool* is shared across classes, but each class's own
    // known/prepared budget must be spent from that class's own pool —
    // `attribute_multiclass_choices` assigns the submitted picks back to
    // whichever class's pool/budget they satisfy.
    let mut cantrip_pools: Vec<Vec<String>> = Vec::new();
    let mut spell_pools: Vec<Vec<String>> = Vec::new();
    let mut cantrip_required: Vec<usize> = Vec::new();
    let mut spell_required: Vec<usize> = Vec::new();
    let mut allowed_spells: Vec<Spell> = Vec::new();
    for (entry, detail, subclass) in &resolved {
        let profile = effective_spellcasting(&detail.class, *subclass);
        let max_spell_level =
            max_castable_spell_level(profile.caster_progression.as_deref().unwrap_or(""), entry.level);
        let entry_spells = crate::db::list_spells_for_class(
            &pool,
            &entry.class_id,
            subclass.map(|s| s.id.as_str()),
            entry.level,
            max_spell_level,
        )
        .await
        .map_err(|err| ServerFnError::new(err.to_string()))?;
        cantrip_pools.push(entry_spells.iter().filter(|s| s.level == 0).map(|s| s.id.clone()).collect());
        spell_pools.push(entry_spells.iter().filter(|s| s.level > 0).map(|s| s.id.clone()).collect());
        let ability_score = profile
            .spellcasting_ability
            .as_deref()
            .and_then(|code| finals.iter().find(|(c, _)| *c == code).map(|(_, score)| *score))
            .unwrap_or(10);
        cantrip_required.push(cantrips_known_count(&profile, entry.level).min(cantrip_pools.last().unwrap().len()));
        spell_required.push(spells_known_count(&profile, entry.level, ability_score).min(spell_pools.last().unwrap().len()));
        allowed_spells.extend(entry_spells);
    }

    if !attribute_multiclass_choices(&character.cantrip_choices, &cantrip_pools, &cantrip_required) {
        return Err(ServerFnError::new(
            "Cantrip choices don't match what's unlocked across this character's classes",
        ));
    }
    if !attribute_multiclass_choices(&character.spell_choices, &spell_pools, &spell_required) {
        return Err(ServerFnError::new(
            "Spell choices don't match what's unlocked across this character's classes",
        ));
    }

    let mut spell_choice_slots: Vec<Vec<String>> = Vec::new();
    let mut spell_choice_pool_spells: Vec<Spell> = Vec::new();
    for (entry, _detail, subclass) in &resolved {
        if let Some(subclass) = subclass {
            let pools = crate::db::get_subclass_spell_choice_pools(&pool, &subclass.id, entry.level)
                .await
                .map_err(|err| ServerFnError::new(err.to_string()))?;
            for (count, spells) in pools {
                if spells.is_empty() {
                    continue;
                }
                let ids: Vec<String> = spells.iter().map(|s| s.id.clone()).collect();
                for _ in 0..count {
                    spell_choice_slots.push(ids.clone());
                }
                spell_choice_pool_spells.extend(spells);
            }
        }
    }
    let mut seen_spell_grant_choices = HashSet::new();
    let spell_grant_choices_valid = character.spell_grant_choices.len() == spell_choice_slots.len()
        && character.spell_grant_choices.iter().zip(&spell_choice_slots).all(|(choice, pool)| {
            pool.contains(choice)
                && seen_spell_grant_choices.insert(choice)
                && !character.cantrip_choices.contains(choice)
                && !character.spell_choices.contains(choice)
        });
    if !spell_grant_choices_valid {
        return Err(ServerFnError::new(
            "Subclass spell choices don't match what's unlocked for this character's classes",
        ));
    }

    // Same reasoning again, for optional features (Invocations, Fighting
    // Style, Maneuvers, ...): re-derive the active FeatureTypes, their
    // quotas, and each candidate's eligibility (level/class/subclass/pact/
    // known-spell prerequisites) — each class's own eligibility is checked
    // against that class's own name/source/subclass/level, same as a
    // single-class character — from the DB-fetched classes/subclasses and
    // the character's own submitted choices, so a direct POST can't persist
    // a pick the wizard wouldn't have offered, including one gated on a Pact
    // Boon the character didn't actually choose.
    let mut known_spell_names: HashSet<String> = allowed_spells
        .iter()
        .filter(|spell| {
            character.cantrip_choices.contains(&spell.id) || character.spell_choices.contains(&spell.id)
        })
        .map(|spell| spell.name.to_lowercase())
        .collect();
    known_spell_names.extend(
        spell_choice_pool_spells
            .iter()
            .filter(|spell| character.spell_grant_choices.contains(&spell.id))
            .map(|spell| spell.name.to_lowercase()),
    );
    for (entry, _detail, subclass) in &resolved {
        if let Some(subclass) = subclass {
            let granted = crate::db::get_subclass_granted_spells(&pool, &subclass.id, entry.level)
                .await
                .map_err(|err| ServerFnError::new(err.to_string()))?;
            known_spell_names.extend(granted.iter().map(|s| s.name.to_lowercase()));
        }
    }

    let entries_for_quota: Vec<(&Class, Option<&Subclass>, u8)> =
        resolved.iter().map(|(entry, detail, subclass)| (&detail.class, *subclass, entry.level)).collect();
    let active_types = multiclass_active_optional_feature_types(&entries_for_quota);

    if !active_types.is_empty() {
        let candidates = crate::db::list_optional_features(
            &pool,
            &OptionalFeatureQuery { types: active_types.clone(), ..Default::default() },
        )
        .await
        .map_err(|err| ServerFnError::new(err.to_string()))?;

        let chosen_feature_names: HashSet<String> = candidates
            .iter()
            .filter(|f| character.optional_feature_choices.contains(&f.id))
            .map(|f| f.name.to_lowercase())
            .collect();

        let mut all_eligible_ids: HashSet<&str> = HashSet::new();
        let mut total_required = 0usize;
        for (entry, detail, subclass) in &resolved {
            let ctx = EligibilityContext {
                level: entry.level,
                class_name: &detail.class.name,
                class_source: &detail.class.source,
                subclass_name: subclass.map(|s| s.name.as_str()),
                subclass_source: subclass.map(|s| s.source.as_str()),
                known_spell_names: &known_spell_names,
                chosen_feature_names: &chosen_feature_names,
            };
            // This entry's own active types, not the globally-active set —
            // a feature that carries no class-specific prerequisite (its
            // class-scoping comes entirely from which FeatureType(s) it's
            // tagged with, e.g. most Fighting Styles) reads as "eligible"
            // under any class's `EligibilityContext`, so checking it against
            // a type this entry doesn't actually grant would force
            // `required` to 0 while `chosen_count` still counts it,
            // wrongly rejecting an otherwise-valid pick made under a
            // different class's own slot.
            for &feature_type in &active_optional_feature_types(&detail.class, *subclass, entry.level) {
                let eligible: Vec<&crate::models::optional_feature::OptionalFeature> = candidates
                    .iter()
                    .filter(|f| f.feature_types.contains(&feature_type) && is_eligible(&f.prerequisites, &ctx))
                    .collect();
                let quota = optional_feature_quota(&detail.class, *subclass, feature_type, entry.level);
                let required = quota.min(eligible.len());
                total_required += required;
                let chosen_count = character
                    .optional_feature_choices
                    .iter()
                    .filter(|id| eligible.iter().any(|f| &f.id == *id))
                    .count();
                if chosen_count != required {
                    return Err(ServerFnError::new(
                        "Optional feature choices don't match what's unlocked, ineligible, or a pact prerequisite for this character's classes",
                    ));
                }
                all_eligible_ids.extend(eligible.iter().map(|f| f.id.as_str()));
            }
        }
        let choices_valid = character.optional_feature_choices.len() == total_required
            && character.optional_feature_choices.iter().all(|id| all_eligible_ids.contains(id.as_str()));
        if !choices_valid {
            return Err(ServerFnError::new(
                "Optional feature choices don't match what's unlocked, ineligible, or a pact prerequisite for this character's classes",
            ));
        }
    } else if !character.optional_feature_choices.is_empty() {
        return Err(ServerFnError::new(
            "Optional feature choices don't match what's unlocked for this character's classes",
        ));
    }

    let prereq_violations = multiclass_prereq_violations(&character, &class_lookup, Some(&species));
    if !prereq_violations.is_empty() {
        return Err(ServerFnError::new(prereq_violations.join("; ")));
    }

    // Persist each entry's server-derived subclass_id, not whatever the POST
    // sent — otherwise a direct POST could still get a premature subclass_id
    // stored even though the validation above correctly ignored it, and that
    // dangling value would then resurface unchecked on the character sheet.
    let final_subclass_ids: Vec<Option<String>> =
        resolved.iter().map(|(_, _, subclass)| subclass.map(|s| s.id.clone())).collect();

    let mut character = character;
    for (entry, subclass_id) in character.classes.iter_mut().zip(final_subclass_ids) {
        entry.subclass_id = subclass_id;
    }
    if character.id.is_empty() {
        character.id = uuid::Uuid::new_v4().to_string();
        character.user_id = user.id;
    } else {
        // The wizard's form model has no user_id field, so re-saving an
        // existing character always arrives with it blank. Reload it scoped
        // to the caller's own id — if that comes back empty, either the
        // character doesn't exist or belongs to someone else, so reject
        // rather than let the save silently reassign or fabricate a row.
        let existing = crate::db::get_character(&pool, &character.id, &user.id)
            .await
            .map_err(|err| ServerFnError::new(err.to_string()))?;
        let Some(existing) = existing else {
            return Err(ServerFnError::new("character not found"));
        };
        character.user_id = existing.user_id;
    }
    crate::db::save_character(&pool, &character)
        .await
        .map_err(|err| ServerFnError::new(err.to_string()))?;
    Ok(character.id)
}

const STEPS: [&str; 12] = [
    "Basics",
    "Class",
    "Optional Features",
    "Species",
    "Background",
    "Skills",
    "Languages",
    "Tools",
    "Abilities",
    "Spells",
    "Feats & ASIs",
    "Review",
];

// One added class, resolved against its own `ClassDetail` — `index` mirrors its position in `classes`.
#[derive(Clone, PartialEq)]
struct ResolvedClassEntry {
    entry: ClassLevel,
    index: usize,
    detail: ClassDetail,
    subclass: Option<Subclass>,
    spellcasting: SpellcastingProfile,
    max_spell_level: u8,
    active_types: Vec<FeatureType>,
}

// One (class, FeatureType) slot — e.g. a Fighter/Paladin multiclass gets two independent Fighting Style slots, not one merged one.
#[derive(Clone, PartialEq)]
struct FeatureTypeSlot {
    class_id: String,
    feature_type: FeatureType,
    eligible_ids: HashSet<String>,
    required: usize,
}

// Cheap serde-able stand-in for a `ResolvedClassEntry`, used only for Resource staleness checks.
type EntryKey = (String, Option<String>, u8);
fn entry_key(entry: &ResolvedClassEntry) -> EntryKey {
    (entry.entry.class_id.clone(), entry.subclass.as_ref().map(|s| s.id.clone()), entry.entry.level)
}

// One caster class's own required cantrip/spell counts — see `spell_requirements`.
#[derive(Clone, PartialEq)]
struct SpellRequirement {
    class_id: String,
    cantrips_required: usize,
    spells_required: usize,
}

/// One required pick slot from a subclass's `{"choose": ...}` spell grant —
/// a `count: 2` filter expands into two slots.
#[derive(Clone, PartialEq)]
struct SpellChoiceSlot {
    class_id: String,
    source: String,
    options: Vec<Spell>,
}

// `classes[0]` is brass, `classes[1]` teal, `classes[2]`+ ember — see `.class-band-N` in style/main.css.
fn class_band_class(index: usize) -> &'static str {
    match index % 3 {
        0 => "class-band-0",
        1 => "class-band-1",
        _ => "class-band-2",
    }
}

#[component]
pub fn CharacterBuilderPage() -> impl IntoView {
    let params = use_params_map();

    let step = RwSignal::new(0usize);
    let character_id = RwSignal::new(String::new());
    let name = RwSignal::new(String::new());
    // classes[0] is implicitly the primary class (pick order), same
    // convention as `hp_rolls` — see `ClassLevel`'s own doc comment.
    let classes = RwSignal::new(Vec::<ClassLevel>::new());
    let char_level = Memo::new(move |_| total_level(&classes.get()));
    // Which class's sub-tab is focused in each step that splits its content
    // per class once there's more than one (Skills stays one combined list —
    // see `skill_inputs`'s doc comment) — lifted here (not local to each step
    // function) so it survives that step being torn down and rebuilt on
    // navigation, same reasoning as `species_active_group` below.
    let spells_class_tab = RwSignal::new(0usize);
    let features_class_tab = RwSignal::new(0usize);
    let asi_class_tab = RwSignal::new(0usize);
    let species_id = RwSignal::new(None::<String>);
    let background_id = RwSignal::new(None::<String>);
    let ability_method = RwSignal::new(AbilityMethod::StandardArray);
    let abilities = RwSignal::new(AbilityScores {
        strength: 15,
        dexterity: 14,
        constitution: 13,
        intelligence: 12,
        wisdom: 10,
        charisma: 8,
    });
    let asi_choices = RwSignal::new(Vec::<Option<AsiChoice>>::new());
    let skill_choices = RwSignal::new(Vec::<Option<String>>::new());
    let expertise_choices = RwSignal::new(Vec::<Option<String>>::new());
    let language_choices = RwSignal::new(Vec::<Option<String>>::new());
    let tool_choices = RwSignal::new(Vec::<Option<String>>::new());
    let custom_skill_choices = RwSignal::new(Vec::<Option<String>>::new());
    let custom_language_choices = RwSignal::new(Vec::<Option<String>>::new());
    let custom_tool_choices = RwSignal::new(Vec::<Option<String>>::new());
    let cantrip_choices = RwSignal::new(Vec::<String>::new());
    let spell_choices = RwSignal::new(Vec::<String>::new());
    let spell_grant_choices = RwSignal::new(Vec::<Option<String>>::new());
    let optional_feature_choices = RwSignal::new(Vec::<String>::new());
    let species_ability_choices = RwSignal::new(Vec::<String>::new());
    let ability_bonus_source = RwSignal::new(AbilityBonusSource::default());
    let custom_ability_bonus_choices = RwSignal::new(Vec::<String>::new());
    let hp_method = RwSignal::new(HpMethod::Average);
    let hp_rolls = RwSignal::new(Vec::<u8>::new());
    let hp_manual = RwSignal::new(None::<i32>);
    let enforce_prereqs = RwSignal::new(true);

    let species_search = RwSignal::new(String::new());
    let species_active_group = RwSignal::new(None::<String>);
    let background_search = RwSignal::new(String::new());

    let class_list = Resource::new(|| (), |_| async move { get_classes().await });
    // ids alone (not the full `ClassLevel`s) so a level/subclass edit doesn't
    // refetch every class's detail — only adding/removing/swapping a class
    // does.
    let class_ids = Memo::new(move |_| classes.get().iter().map(|c| c.class_id.clone()).collect::<Vec<_>>());
    // One combined resource for every added class's detail, keyed on the
    // full id list — echoes the ids back alongside the results (same
    // staleness-detection pattern as `spell_options` below) so
    // `class_details_ready` can tell "resolved for the current class list"
    // apart from "still serving detail for a class list we've since edited".
    let class_details: Resource<Result<(Vec<String>, Vec<(String, ClassDetail)>), ServerFnError>> = Resource::new(
        move || class_ids.get(),
        |ids| async move {
            let mut resolved = Vec::new();
            for id in &ids {
                if let Some(detail) = get_class_detail(id.clone()).await? {
                    resolved.push((id.clone(), detail));
                }
            }
            Ok((ids, resolved))
        },
    );
    let class_details_ready =
        move || class_details.get().and_then(|r| r.ok()).is_some_and(|(ids, _)| ids == class_ids.get());
    let class_details_map = move || -> HashMap<String, ClassDetail> {
        class_details.get().and_then(|r| r.ok()).map(|(_, list)| list.into_iter().collect()).unwrap_or_default()
    };

    // Every added class, resolved into its own `ClassDetail` + active
    // subclass + spellcasting picture + optional-feature types — the one
    // derived structure every per-class sub-tab (Skills, Spells, Optional
    // Features, Feats & ASIs) and the Class step itself index into, instead
    // of each step re-deriving this from `classes`/`class_details` on its
    // own.
    let resolved_entries = Memo::new(move |_| {
        let details = class_details_map();
        classes
            .get()
            .into_iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                let detail = details.get(&entry.class_id)?.clone();
                let subclass = active_subclass(&detail, entry.subclass_id.as_deref(), entry.level).cloned();
                let spellcasting = effective_spellcasting(&detail.class, subclass.as_ref());
                let max_spell_level = spellcasting
                    .caster_progression
                    .as_ref()
                    .map(|progression| max_castable_spell_level(progression, entry.level))
                    .unwrap_or(0);
                let active_types = active_optional_feature_types(&detail.class, subclass.as_ref(), entry.level);
                Some(ResolvedClassEntry { entry, index, detail, subclass, spellcasting, max_spell_level, active_types })
            })
            .collect::<Vec<_>>()
    });
    let species_list = Resource::new(
        move || species_search.get(),
        |search| async move {
            get_species(SpeciesQuery { search, ..Default::default() }).await
        },
    );
    // Keyed on species_id alone, same reasoning as background_detail below —
    // the Review step needs the chosen species' own name/source independent
    // of whatever's currently in the step-2 search box.
    let species_detail = Resource::new(
        move || species_id.get(),
        |id| async move {
            match id {
                Some(id) => get_species_by_id(id).await,
                None => Ok(None),
            }
        },
    );
    let background_list = Resource::new(
        move || background_search.get(),
        |search| async move {
            get_backgrounds(BackgroundQuery { search, ..Default::default() }).await
        },
    );
    // Keyed on background_id alone (not the step-3 search box), same
    // reasoning as species_ability_hint — the Skills step needs the chosen
    // background's data independent of later edits to the search text.
    let background_detail = Resource::new(
        move || background_id.get(),
        |id| async move {
            match id {
                Some(id) => get_background(id).await,
                None => Ok(None),
            }
        },
    );
    let feat_list = Resource::new(|| (), |_| async move { get_feats(FeatQuery::default()).await });

    let prefill = Resource::new(
        move || params.read().get("id").unwrap_or_default(),
        |id| async move {
            if id.is_empty() {
                Ok(None)
            } else {
                get_character(id).await
            }
        },
    );
    let prefilled = RwSignal::new(false);
    Effect::new(move |_| {
        if prefilled.get_untracked() {
            return;
        }
        if let Some(Ok(Some(existing))) = prefill.get() {
            character_id.set(existing.id);
            name.set(existing.name);
            classes.set(existing.classes);
            species_id.set(Some(existing.species_id));
            background_id.set(Some(existing.background_id));
            ability_method.set(existing.ability_method);
            abilities.set(existing.abilities);
            asi_choices.set(existing.asi_choices);
            skill_choices.set(existing.skill_choices.into_iter().map(Some).collect());
            expertise_choices.set(existing.expertise_choices.into_iter().map(Some).collect());
            language_choices.set(existing.language_choices.into_iter().map(Some).collect());
            tool_choices.set(existing.tool_choices.into_iter().map(Some).collect());
            custom_skill_choices.set(existing.custom_skill_proficiencies.into_iter().map(Some).collect());
            custom_language_choices.set(existing.custom_language_proficiencies.into_iter().map(Some).collect());
            custom_tool_choices.set(existing.custom_tool_proficiencies.into_iter().map(Some).collect());
            cantrip_choices.set(existing.cantrip_choices);
            spell_choices.set(existing.spell_choices);
            spell_grant_choices.set(existing.spell_grant_choices.into_iter().map(Some).collect());
            optional_feature_choices.set(existing.optional_feature_choices);
            species_ability_choices.set(existing.species_ability_choices);
            ability_bonus_source.set(existing.ability_bonus_source);
            custom_ability_bonus_choices.set(existing.custom_ability_bonus_choices);
            hp_method.set(existing.hp_method);
            hp_rolls.set(existing.hp_rolls);
            hp_manual.set(existing.hp_manual);
            enforce_prereqs.set(existing.enforce_multiclass_prereqs);
            prefilled.set(true);
        }
    });

    // Every added class's features, keyed by class_id — the shared input the
    // `multiclass_*` slot-count derive functions all take.
    let features_by_class = move || -> HashMap<String, Vec<crate::models::class::ClassFeature>> {
        resolved_entries.get().into_iter().map(|e| (e.entry.class_id, e.detail.features)).collect()
    };

    let build_character = move || {
        let details = class_details_map();
        let classes_final: Vec<ClassLevel> = classes
            .get()
            .into_iter()
            .map(|c| {
                let unlocked =
                    details.get(&c.class_id).and_then(subclass_unlock_level).is_some_and(|unlock| unlock <= c.level);
                ClassLevel { subclass_id: if unlocked { c.subclass_id } else { None }, ..c }
            })
            .collect();
        Character {
            id: character_id.get(),
            // Resolved server-side from the session in `save_character` —
            // the wizard never tracks or displays ownership itself.
            user_id: String::new(),
            name: name.get().trim().to_string(),
            species_id: species_id.get().unwrap_or_default(),
            background_id: background_id.get().unwrap_or_default(),
            classes: classes_final,
            ability_method: ability_method.get(),
            abilities: abilities.get(),
            asi_choices: asi_choices.get(),
            skill_choices: skill_choices.get().into_iter().flatten().collect(),
            expertise_choices: expertise_choices.get().into_iter().flatten().collect(),
            language_choices: language_choices.get().into_iter().flatten().collect(),
            tool_choices: tool_choices.get().into_iter().flatten().collect(),
            custom_skill_proficiencies: custom_skill_choices.get().into_iter().flatten().collect(),
            custom_language_proficiencies: custom_language_choices.get().into_iter().flatten().collect(),
            custom_tool_proficiencies: custom_tool_choices.get().into_iter().flatten().collect(),
            cantrip_choices: cantrip_choices.get(),
            spell_choices: spell_choices.get(),
            spell_grant_choices: spell_grant_choices.get().into_iter().flatten().collect(),
            optional_feature_choices: optional_feature_choices.get(),
            species_ability_choices: species_ability_choices.get(),
            ability_bonus_source: ability_bonus_source.get(),
            custom_ability_bonus_choices: custom_ability_bonus_choices.get(),
            hp_method: hp_method.get(),
            hp_rolls: hp_rolls.get(),
            hp_manual: hp_manual.get(),
            enforce_multiclass_prereqs: enforce_prereqs.get(),
        }
    };

    // Feeds the Class step's soft prerequisite warning; Save/Review is the real gate.
    let current_final_abilities = move || {
        let species = species_detail.get().and_then(|result| result.ok()).flatten();
        final_abilities(&build_character(), species.as_ref())
    };

    // Every unlocked ASI/feat slot across every class, grouped contiguously
    // by which class granted it (in `classes` add-order) — `save_character`
    // only checks the total count (`multiclass_asi_slots`), so this ordering
    // is a pure UI concern: it's what makes each class's own slots a
    // contiguous sub-slice of the flat `asi_choices` vec, for the Feats &
    // ASIs step's per-class sub-tabs.
    // Stores tuples of (entry_index, class_id, asi_level) for both tab logic
    // and stable reorganization when classes change.
    let asi_entries = Memo::new(move |_| {
        resolved_entries
            .get()
            .into_iter()
            .flat_map(|e| {
                let idx = e.index;
                let class_id = e.entry.class_id.clone();
                asi_levels(&e.detail.features)
                    .into_iter()
                    .filter(move |asi_level| *asi_level <= e.entry.level)
                    .map(move |asi_level| (idx, class_id.clone(), asi_level))
            })
            .collect::<Vec<(usize, String, u8)>>()
    });
    // Reconcile asi_choices with the new slot structure when classes
    // are reordered/added/removed. Build a stable (class_id, asi_level) -> choice
    // map from the old vec, then rebuild the vec in the new order.
    Effect::new(move |_| {
        let new_slots = asi_entries.get();
        asi_choices.update(|choices| {
            // Build a stable map of (class_id, asi_level) -> stored choice
            let old_entries = resolved_entries.get();
            let old_flat: Vec<(String, u8)> = old_entries
                .into_iter()
                .flat_map(|e| {
                    let class_id = e.entry.class_id.clone();
                    asi_levels(&e.detail.features)
                        .into_iter()
                        .filter(move |asi_level| *asi_level <= e.entry.level)
                        .map(move |asi_level| (class_id.clone(), asi_level))
                })
                .collect();
            let mut stable_map: HashMap<(String, u8), Option<AsiChoice>> = HashMap::new();
            let old_choices = choices.clone();
            for (slot_idx, old_choice) in old_choices.into_iter().enumerate() {
                if slot_idx < old_flat.len() {
                    let (class_id, asi_level) = &old_flat[slot_idx];
                    stable_map.insert((class_id.clone(), *asi_level), old_choice);
                }
            }
            // Rebuild vec in new order, pulling from stable map
            let mut new_choices = Vec::new();
            for (_, class_id, asi_level) in new_slots.iter() {
                new_choices.push(stable_map.remove(&(class_id.clone(), *asi_level)).flatten());
            }
            *choices = new_choices;
        });
    });

    // One roll slot per total level after 1st (level 1 is always max hit
    // die, never rolled) — kept in sync with `char_level` regardless of
    // which `hp_method` is currently active, the same way `asi_choices`
    // above tracks its own level-derived slot count. Stored with stable keys
    // (class_id, class_level_in_class) to preserve rolls when classes are
    // reordered: a roll for Fighter's level 2 stays a Fighter level 2 roll.
    let hp_entries = Memo::new(move |_| {
        resolved_entries
            .get()
            .into_iter()
            .enumerate()
            .flat_map(|(class_idx, e)| {
                let class_id = e.entry.class_id.clone();
                let start = if class_idx == 0 { 2 } else { 1 };
                (start..=e.entry.level)
                    .map(move |class_level| (class_id.clone(), class_level))
            })
            .collect::<Vec<(String, u8)>>()
    });
    // Reconcile hp_rolls with reordered/added/removed classes.
    Effect::new(move |_| {
        let new_levels = hp_entries.get();
        hp_rolls.update(|rolls| {
            // Build stable map from old rolls keyed by (class_id, class_level)
            let old_entries = resolved_entries.get();
            let mut stable_map: HashMap<(String, u8), u8> = HashMap::new();
            let old_rolls = rolls.clone();
            let mut roll_idx = 0;
            for (class_idx, e) in old_entries.iter().enumerate() {
                let class_id = e.entry.class_id.clone();
                let start = if class_idx == 0 { 2 } else { 1 };
                for class_level in start..=e.entry.level {
                    if roll_idx < old_rolls.len() {
                        stable_map.insert((class_id.clone(), class_level), old_rolls[roll_idx]);
                        roll_idx += 1;
                    }
                }
            }
            // Rebuild vec in new order from stable map
            let new_rolls: Vec<u8> = new_levels
                .iter()
                .map(|(class_id, class_level)| {
                    stable_map.get(&(class_id.clone(), *class_level)).copied().unwrap_or(0)
                })
                .collect();
            *rolls = new_rolls;
        });
    });

    // Skill choice slots are a single combined pool across every class
    // (5e doesn't segregate multiclass skill picks per class the way it does
    // spells/optional features — each grant still keeps its own specific
    // option list per resulting slot, `multiclass_class_skill_grants`
    // preserves that, it's just not split into per-class UI sections).
    let skill_inputs = Memo::new(move |_| {
        let class_lookup: HashMap<String, Class> =
            class_details_map().into_iter().map(|(id, detail)| (id, detail.class)).collect();
        let combined_class_skills = multiclass_class_skill_grants(&classes.get(), &class_lookup);
        let background_skills: Vec<(String, SkillGrant)> = background_detail
            .get()
            .and_then(|result| result.ok())
            .flatten()
            .map(|b| b.skills.iter().cloned().map(|grant| (b.name.clone(), grant)).collect())
            .unwrap_or_default();
        skill_slots(&combined_class_skills, &background_skills)
    });
    let skill_proficiency_sources = Memo::new(move |_| -> Vec<(String, Vec<String>)> {
        let class_lookup: HashMap<String, Class> =
            class_details_map().into_iter().map(|(id, detail)| (id, detail.class)).collect();
        let combined_class_skills = multiclass_class_skill_grants(&classes.get(), &class_lookup);
        let background_skills: Vec<(String, SkillGrant)> = background_detail
            .get()
            .and_then(|result| result.ok())
            .flatten()
            .map(|b| b.skills.iter().cloned().map(|grant| (b.name.clone(), grant)).collect())
            .unwrap_or_default();

        let mut sources: HashMap<String, Vec<String>> = HashMap::new();
        for (source, grant) in combined_class_skills.iter().chain(background_skills.iter()) {
            if let SkillGrant::Fixed { skills } = grant {
                for skill in skills {
                    let entry = sources.entry(skill.clone()).or_default();
                    if !entry.contains(source) {
                        entry.push(source.clone());
                    }
                }
            }
        }
        let slots = skill_inputs.get();
        for (choice, pool) in skill_choices.get().iter().zip(slots.choice_pools.iter()) {
            if let Some(skill) = choice {
                let entry = sources.entry(skill.clone()).or_default();
                if !entry.contains(&pool.source) {
                    entry.push(pool.source.clone());
                }
            }
        }
        for skill in custom_skill_choices.get().into_iter().flatten() {
            let entry = sources.entry(skill).or_default();
            if !entry.iter().any(|source| source == "Custom") {
                entry.push("Custom".to_string());
            }
        }
        let mut rows: Vec<(String, Vec<String>)> = sources.into_iter().collect();
        rows.sort_by(|a, b| skill_label(&a.0).cmp(&skill_label(&b.0)));
        rows
    });

    // Languages are DB-backed (unlike skills' compile-time `SKILLS`), so the
    // full reference list is fetched once and reused as the fallback pool.
    let language_list =
        Resource::new(|| (), |_| async move { get_languages().await });
    let all_language_names = move || -> Vec<String> {
        language_list.get().and_then(|r| r.ok()).map(|langs| langs.into_iter().map(|l| l.name).collect()).unwrap_or_default()
    };
    let language_inputs = Memo::new(move |_| {
        let background_languages: Vec<(String, LanguageGrant)> = background_detail
            .get()
            .and_then(|result| result.ok())
            .flatten()
            .map(|b| b.languages.iter().cloned().map(|grant| (b.name.clone(), grant)).collect())
            .unwrap_or_default();
        let species_languages: Vec<(String, LanguageGrant)> = species_detail
            .get()
            .and_then(|result| result.ok())
            .flatten()
            .map(|s| s.languages.iter().cloned().map(|grant| (s.name.clone(), grant)).collect())
            .unwrap_or_default();
        language_slots(&background_languages, &species_languages, &all_language_names())
    });
    let language_proficiency_sources = Memo::new(move |_| -> Vec<(String, Vec<String>)> {
        let background_languages: Vec<(String, LanguageGrant)> = background_detail
            .get()
            .and_then(|result| result.ok())
            .flatten()
            .map(|b| b.languages.iter().cloned().map(|grant| (b.name.clone(), grant)).collect())
            .unwrap_or_default();
        let species_languages: Vec<(String, LanguageGrant)> = species_detail
            .get()
            .and_then(|result| result.ok())
            .flatten()
            .map(|s| s.languages.iter().cloned().map(|grant| (s.name.clone(), grant)).collect())
            .unwrap_or_default();

        let mut sources: HashMap<String, Vec<String>> = HashMap::new();
        for (source, grant) in background_languages.iter().chain(species_languages.iter()) {
            if let LanguageGrant::Fixed { languages } = grant {
                for language in languages {
                    let entry = sources.entry(language.clone()).or_default();
                    if !entry.contains(source) {
                        entry.push(source.clone());
                    }
                }
            }
        }
        let slots = language_inputs.get();
        for (choice, pool) in language_choices.get().iter().zip(slots.choice_pools.iter()) {
            if let Some(language) = choice {
                let entry = sources.entry(language.clone()).or_default();
                if !entry.contains(&pool.source) {
                    entry.push(pool.source.clone());
                }
            }
        }
        for language in custom_language_choices.get().into_iter().flatten() {
            let entry = sources.entry(language).or_default();
            if !entry.iter().any(|source| source == "Custom") {
                entry.push("Custom".to_string());
            }
        }
        let mut rows: Vec<(String, Vec<String>)> = sources.into_iter().collect();
        rows.sort_by(|a, b| title_case(&a.0).cmp(&title_case(&b.0)));
        rows
    });

    // Real tool item names per category, resolved from the `items` table —
    // fetched once and reused to expand `ToolOption::Category`/`AnyCategory`
    // grants into concrete choosable options.
    let tool_category_members = Resource::new(|| (), |_| async move { get_tool_category_members().await });
    let category_members_map = move || -> HashMap<ToolCategory, Vec<String>> {
        tool_category_members.get().and_then(|r| r.ok()).map(|pairs| pairs.into_iter().collect()).unwrap_or_default()
    };
    let all_tool_names = move || -> Vec<String> {
        let mut names: Vec<String> = category_members_map().values().flatten().cloned().collect();
        names.sort();
        names.dedup();
        names
    };
    let tool_inputs = Memo::new(move |_| {
        let class_lookup: HashMap<String, Class> =
            class_details_map().into_iter().map(|(id, detail)| (id, detail.class)).collect();
        let combined_class_tools = multiclass_class_tool_grants(&classes.get(), &class_lookup);
        let background_tools: Vec<(String, ToolGrant)> = background_detail
            .get()
            .and_then(|result| result.ok())
            .flatten()
            .map(|b| b.tools.iter().cloned().map(|grant| (b.name.clone(), grant)).collect())
            .unwrap_or_default();
        tool_slots(&combined_class_tools, &background_tools, &category_members_map())
    });
    let tool_proficiency_sources = Memo::new(move |_| -> Vec<(String, Vec<String>)> {
        let class_lookup: HashMap<String, Class> =
            class_details_map().into_iter().map(|(id, detail)| (id, detail.class)).collect();
        let combined_class_tools = multiclass_class_tool_grants(&classes.get(), &class_lookup);
        let background_tools: Vec<(String, ToolGrant)> = background_detail
            .get()
            .and_then(|result| result.ok())
            .flatten()
            .map(|b| b.tools.iter().cloned().map(|grant| (b.name.clone(), grant)).collect())
            .unwrap_or_default();

        let mut sources: HashMap<String, Vec<String>> = HashMap::new();
        for (source, grant) in combined_class_tools.iter().chain(background_tools.iter()) {
            if let ToolGrant::Fixed { tools } = grant {
                for tool in tools {
                    let entry = sources.entry(tool.clone()).or_default();
                    if !entry.contains(source) {
                        entry.push(source.clone());
                    }
                }
            }
        }
        let slots = tool_inputs.get();
        for (choice, pool) in tool_choices.get().iter().zip(slots.choice_pools.iter()) {
            if let Some(tool) = choice {
                let entry = sources.entry(tool.clone()).or_default();
                if !entry.contains(&pool.source) {
                    entry.push(pool.source.clone());
                }
            }
        }
        for tool in custom_tool_choices.get().into_iter().flatten() {
            let entry = sources.entry(tool).or_default();
            if !entry.iter().any(|source| source == "Custom") {
                entry.push("Custom".to_string());
            }
        }
        let mut rows: Vec<(String, Vec<String>)> = sources.into_iter().collect();
        rows.sort_by(|a, b| title_case(&a.0).cmp(&title_case(&b.0)));
        rows
    });

    let background_ready = move || match background_id.get() {
        None => true,
        Some(id) => background_detail
            .get()
            .and_then(|result| result.ok())
            .flatten()
            .is_some_and(|b| b.id == id),
    };
    let species_ready = move || match species_id.get() {
        None => true,
        Some(id) => {
            species_detail.get().and_then(|result| result.ok()).flatten().is_some_and(|s| s.id == id)
        }
    };

    // The species' one Choose-ability-bonus grant, if it has one (see
    // `species_ability_choices`'s doc comment for the at-most-one assumption).
    let species_choose_grant = move || {
        species_detail.get().and_then(|result| result.ok()).flatten().and_then(|species| {
            species.ability_bonuses.into_iter().find_map(|grant| match grant {
                AbilityBonusGrant::Choose { count, amount, from } => Some((count, amount, from)),
                AbilityBonusGrant::Fixed { .. } => None,
            })
        })
    };

    // Resizes to the current slot count AND clears any entry that's no
    // longer a member of its slot's (possibly just-recomputed) pool —
    // otherwise a stale pick from before a class/background change would
    // silently survive and only get caught by save_character's server-side
    // re-validation, surfacing as a generic save error instead of being
    // reset at the step where the inconsistency was introduced.
    //
    // Guarded on both resources actually reflecting the current selection —
    // resizing against a still-stale/empty computation would wipe real
    // picks (prefilled or just-made) down to nothing, then re-extend with
    // `None` once the real data lands, losing them for good instead of
    // merely reordering a re-render.
    Effect::new(move |_| {
        if !class_details_ready() || !background_ready() {
            return;
        }
        let slots = skill_inputs.get();
        let pools = slots.choice_pools;
        skill_choices.update(|choices| {
            choices.resize(pools.len(), None);
            for (choice, pool) in choices.iter_mut().zip(pools.iter()) {
                // Also clears a pick that a background/class change just made
                // redundant (now covered by a fixed grant) — matches
                // `skill_slot`'s disabling of already-fixed options, rather
                // than leaving a now-wasted pick silently selected.
                if choice.as_ref().is_some_and(|skill| !pool.options.contains(skill) || slots.fixed.contains(skill)) {
                    *choice = None;
                }
            }
        });
    });

    let expertise_slot_count =
        Memo::new(move |_| multiclass_expertise_slots(&classes.get(), &features_by_class()));
    // Same reasoning as above — also reacts to skill_choices changing, since
    // an expertise pick can go stale purely from a skill choice changing
    // (expertise must name an already-proficient skill), independent of any
    // class/background/level change.
    Effect::new(move |_| {
        if !class_details_ready() || !background_ready() {
            return;
        }
        let count = expertise_slot_count.get();
        let mut proficient = skill_inputs.get().fixed;
        proficient.extend(skill_choices.get().into_iter().flatten());
        proficient.extend(custom_skill_choices.get().into_iter().flatten());
        expertise_choices.update(|choices| {
            choices.resize(count, None);
            for choice in choices.iter_mut() {
                if choice.as_ref().is_some_and(|skill| !proficient.contains(skill)) {
                    *choice = None;
                }
            }
        });
    });

    // Same reasoning as the skill_choices resize effect — a background or
    // species change can add/remove language slots or make a pick redundant.
    Effect::new(move |_| {
        if !background_ready() || !species_ready() {
            return;
        }
        let slots = language_inputs.get();
        let pools = slots.choice_pools;
        language_choices.update(|choices| {
            choices.resize(pools.len(), None);
            for (choice, pool) in choices.iter_mut().zip(pools.iter()) {
                if choice.as_ref().is_some_and(|language| !pool.options.contains(language) || contains_ignore_case(&slots.fixed, language)) {
                    *choice = None;
                }
            }
        });
    });

    // Same reasoning again — a class or background change can add/remove
    // tool slots or make a pick redundant.
    Effect::new(move |_| {
        if !class_details_ready() || !background_ready() {
            return;
        }
        let slots = tool_inputs.get();
        let pools = slots.choice_pools;
        tool_choices.update(|choices| {
            choices.resize(pools.len(), None);
            for (choice, pool) in choices.iter_mut().zip(pools.iter()) {
                if choice.as_ref().is_some_and(|tool| !pool.options.contains(tool) || contains_ignore_case(&slots.fixed, tool)) {
                    *choice = None;
                }
            }
        });
    });

    // Clears/truncates picks that a species change (or a species with no
    // Choose grant at all) has made invalid — same reasoning as the skill
    // choices reset effect above.
    Effect::new(move |_| {
        if !species_ready() {
            return;
        }
        match species_choose_grant() {
            Some((count, _, from)) => {
                species_ability_choices.update(|choices| {
                    choices.retain(|code| is_free_ability_choice(&from) || from.contains(code));
                    choices.truncate(count as usize);
                });
            }
            None => species_ability_choices.update(|choices| choices.clear()),
        }
    });

    // Echoes the resolved-entries snapshot back alongside the spells (rather
    // than returning `Vec<Spell>` alone) so callers can tell "resolved for
    // the current classes/subclasses/levels" apart from "still serving a
    // stale value from before any of those changed" — same problem
    // `class_details_ready` solves for `class_details`, but `Spell` has no
    // class id of its own to compare against.
    let spell_options_all: Resource<Result<(Vec<EntryKey>, Vec<(String, Vec<Spell>)>), ServerFnError>> =
        Resource::new(
            move || resolved_entries.get(),
            |entries| async move {
                let mut out = Vec::new();
                for e in entries.iter().filter(|e| e.spellcasting.caster_progression.is_some()) {
                    let spells = get_class_spell_options(
                        e.entry.class_id.clone(),
                        e.subclass.as_ref().map(|s| s.id.clone()),
                        e.entry.level,
                        e.max_spell_level,
                    )
                    .await?;
                    out.push((e.entry.class_id.clone(), spells));
                }
                let key = entries.iter().map(entry_key).collect();
                Ok((key, out))
            },
        );
    let spell_options_ready = move || {
        let expected: Vec<EntryKey> = resolved_entries.get().iter().map(entry_key).collect();
        spell_options_all.get().and_then(|result| result.ok()).is_some_and(|(keyed, _)| keyed == expected)
    };
    let spell_pool_for = move |class_id: &str| -> Vec<Spell> {
        spell_options_all
            .get()
            .and_then(|result| result.ok())
            .and_then(|(_, list)| list.into_iter().find(|(id, _)| id == class_id).map(|(_, spells)| spells))
            .unwrap_or_default()
    };

    // Spells each entry's active subclass auto-grants at that entry's own
    // level (already excluded from `spell_options_all` above) — shown
    // read-only so the player can see why a spell they'd expect isn't
    // pickable.
    let granted_spells_all: Resource<Result<Vec<(String, Vec<Spell>)>, ServerFnError>> = Resource::new(
        move || resolved_entries.get(),
        |entries| async move {
            let mut out = Vec::new();
            for e in entries.iter().filter(|e| e.subclass.is_some()) {
                let spells =
                    get_subclass_granted_spells(e.subclass.as_ref().unwrap().id.clone(), e.entry.level).await?;
                out.push((e.entry.class_id.clone(), spells));
            }
            Ok(out)
        },
    );
    let granted_spells_for = move |class_id: &str| -> Vec<Spell> {
        granted_spells_all
            .get()
            .and_then(|result| result.ok())
            .and_then(|list| list.into_iter().find(|(id, _)| id == class_id).map(|(_, spells)| spells))
            .unwrap_or_default()
    };

    let spell_choice_pools_all: Resource<
        Result<(Vec<EntryKey>, Vec<(String, String, Vec<(u8, Vec<Spell>)>)>), ServerFnError>,
    > = Resource::new(
        move || resolved_entries.get(),
        |entries| async move {
            let mut out = Vec::new();
            for e in entries.iter().filter(|e| e.subclass.is_some()) {
                let subclass = e.subclass.as_ref().unwrap();
                let pools = get_subclass_spell_choice_pools(subclass.id.clone(), e.entry.level).await?;
                if !pools.is_empty() {
                    out.push((e.entry.class_id.clone(), subclass.name.clone(), pools));
                }
            }
            let key = entries.iter().map(entry_key).collect();
            Ok((key, out))
        },
    );
    let spell_choice_pools_ready = move || {
        let expected: Vec<EntryKey> = resolved_entries.get().iter().map(entry_key).collect();
        spell_choice_pools_all.get().and_then(|r| r.ok()).is_some_and(|(keyed, _)| keyed == expected)
    };
    let spell_choice_slots = Memo::new(move |_| -> Vec<SpellChoiceSlot> {
        spell_choice_pools_all
            .get()
            .and_then(|r| r.ok())
            .map(|(_, list)| list)
            .unwrap_or_default()
            .into_iter()
            .flat_map(|(class_id, source, pools)| {
                pools.into_iter().filter(|(_, spells)| !spells.is_empty()).flat_map(move |(count, spells)| {
                    let class_id = class_id.clone();
                    let source = source.clone();
                    (0..count).map(move |_| SpellChoiceSlot {
                        class_id: class_id.clone(),
                        source: source.clone(),
                        options: spells.clone(),
                    })
                })
            })
            .collect()
    });
    Effect::new(move |_| {
        if !spell_choice_pools_ready() {
            return;
        }
        let slots = spell_choice_slots.get();
        spell_grant_choices.update(|choices| {
            choices.resize(slots.len(), None);
            for (choice, slot) in choices.iter_mut().zip(slots.iter()) {
                if choice.as_ref().is_some_and(|id| !slot.options.iter().any(|s| &s.id == id)) {
                    *choice = None;
                }
            }
        });
    });

    // Each caster class's own known/prepared spell and cantrip count,
    // against its own level and its own spellcasting ability score —
    // `multiclass_spell_profiles` already does this per 5e's rule that the
    // *slot pool* is shared but each class's own budget isn't.
    let spell_profiles = move || -> Vec<crate::models::character::MulticlassSpellProfile> {
        let character = build_character();
        let species = species_detail.get().and_then(|result| result.ok()).flatten();
        let finals = final_abilities(&character, species.as_ref());
        let lookup: HashMap<String, (Class, Option<Subclass>)> = resolved_entries
            .get()
            .into_iter()
            .map(|e| (e.entry.class_id, (e.detail.class, e.subclass)))
            .collect();
        multiclass_spell_profiles(&classes.get(), &lookup, &finals)
    };
    // One entry per caster class: its own required cantrip/spell counts,
    // already capped by its own pool's availability (`wanted.min(available)`,
    // same rule the single-class wizard always used) — precomputed here
    // (rather than as per-call closures) so it's a single concrete, `Copy`
    // value the Spells step's per-class sub-tabs can be handed directly.
    let spell_requirements = Memo::new(move |_| -> Vec<SpellRequirement> {
        let profiles = spell_profiles();
        resolved_entries
            .get()
            .into_iter()
            .filter(|e| e.spellcasting.caster_progression.is_some())
            .map(|e| {
                let pool = spell_pool_for(&e.entry.class_id);
                let profile = profiles.iter().find(|p| p.class_id == e.entry.class_id);
                let wanted_cantrips = profile.map(|p| p.known_cantrips).unwrap_or(0);
                let wanted_spells = profile.map(|p| p.known_spells).unwrap_or(0);
                let available_cantrips = pool.iter().filter(|spell| spell.level == 0).count();
                let available_spells = pool.iter().filter(|spell| spell.level > 0).count();
                SpellRequirement {
                    class_id: e.entry.class_id,
                    cantrips_required: wanted_cantrips.min(available_cantrips),
                    spells_required: wanted_spells.min(available_spells),
                }
            })
            .collect()
    });
    let total_cantrips_required = move || spell_requirements.get().iter().map(|r| r.cantrips_required).sum::<usize>();
    let total_spells_required = move || spell_requirements.get().iter().map(|r| r.spells_required).sum::<usize>();

    // Clears picks that fell out of the pool entirely (a class/level change
    // narrowed which spells are castable) — same guard-on-ready reasoning as
    // the skill/expertise effects above. Deliberately does NOT also truncate
    // down to the current required count: required count can shrink from an
    // ability-score edit (prepared casters) without any of the player's
    // existing picks becoming invalid pool members, and silently dropping
    // one of their chosen spells in that case would be a surprising,
    // easy-to-miss loss. Instead the Spells step itself shows a banner and
    // blocks Next until the player manually deselects down to the new count.
    Effect::new(move |_| {
        if !class_details_ready() || !spell_options_ready() {
            return;
        }
        let mut cantrip_ids: HashSet<String> = HashSet::new();
        let mut spell_ids: HashSet<String> = HashSet::new();
        for entry in resolved_entries.get() {
            for spell in spell_pool_for(&entry.entry.class_id) {
                if spell.level == 0 {
                    cantrip_ids.insert(spell.id);
                } else {
                    spell_ids.insert(spell.id);
                }
            }
        }
        cantrip_choices.update(|choices| choices.retain(|id| cantrip_ids.contains(id)));
        spell_choices.update(|choices| choices.retain(|id| spell_ids.contains(id)));
    });

    let optional_feature_pool_all: OptionalFeaturePoolResource = Resource::new(
        move || resolved_entries.get(),
        |entries| async move {
            let mut out = Vec::new();
            for e in entries.iter().filter(|e| !e.active_types.is_empty()) {
                let features = get_optional_features(OptionalFeatureQuery {
                    types: e.active_types.clone(),
                    ..Default::default()
                })
                .await?;
                out.push((e.entry.class_id.clone(), features));
            }
            let key = entries.iter().map(entry_key).collect();
            Ok((key, out))
        },
    );
    let optional_feature_pool_ready = move || {
        let expected: Vec<EntryKey> = resolved_entries.get().iter().map(entry_key).collect();
        optional_feature_pool_all.get().and_then(|result| result.ok()).is_some_and(|(keyed, _)| keyed == expected)
    };
    let optional_feature_pool_for = move |class_id: &str| -> Vec<OptionalFeature> {
        optional_feature_pool_all
            .get()
            .and_then(|result| result.ok())
            .and_then(|(_, list)| list.into_iter().find(|(id, _)| id == class_id).map(|(_, features)| features))
            .unwrap_or_default()
    };

    // Lowercased names of every spell currently known/prepared/granted,
    // across every class — must include subclass-auto-granted spells too, or
    // a granted-spell-gated invocation would wrongly read as ineligible.
    let known_spell_names = Memo::new(move |_| {
        let cantrips = cantrip_choices.get();
        let leveled = spell_choices.get();
        let mut names = HashSet::new();
        for entry in resolved_entries.get() {
            for spell in spell_pool_for(&entry.entry.class_id) {
                if cantrips.contains(&spell.id) || leveled.contains(&spell.id) {
                    names.insert(spell.name.to_lowercase());
                }
            }
            for spell in granted_spells_for(&entry.entry.class_id) {
                names.insert(spell.name.to_lowercase());
            }
        }
        for (choice, slot) in spell_grant_choices.get().iter().zip(spell_choice_slots.get().iter()) {
            if let Some(id) = choice {
                if let Some(spell) = slot.options.iter().find(|s| &s.id == id) {
                    names.insert(spell.name.to_lowercase());
                }
            }
        }
        names
    });

    // Lowercased names of optional features already chosen, across every
    // class — the only consumer today is the Pact Boon cross-check (an
    // invocation requiring "pact: Blade" is satisfied once "Pact of the
    // Blade" is in this set, regardless of which class granted either).
    let chosen_feature_names = Memo::new(move |_| {
        let chosen = optional_feature_choices.get();
        let mut names = HashSet::new();
        for entry in resolved_entries.get() {
            for feature in optional_feature_pool_for(&entry.entry.class_id) {
                if chosen.contains(&feature.id) {
                    names.insert(feature.name.to_lowercase());
                }
            }
        }
        names
    });

    let feature_slots = Memo::new(move |_| -> Vec<FeatureTypeSlot> {
        let known_spells = known_spell_names.get();
        let chosen_names = chosen_feature_names.get();
        let mut out = Vec::new();
        for entry in resolved_entries.get() {
            let pool = optional_feature_pool_for(&entry.entry.class_id);
            let ctx = EligibilityContext {
                level: entry.entry.level,
                class_name: &entry.detail.class.name,
                class_source: &entry.detail.class.source,
                subclass_name: entry.subclass.as_ref().map(|s| s.name.as_str()),
                subclass_source: entry.subclass.as_ref().map(|s| s.source.as_str()),
                known_spell_names: &known_spells,
                chosen_feature_names: &chosen_names,
            };
            for &feature_type in &entry.active_types {
                let eligible: Vec<&OptionalFeature> =
                    pool.iter().filter(|f| f.feature_types.contains(&feature_type) && is_eligible(&f.prerequisites, &ctx)).collect();
                let quota = optional_feature_quota(&entry.detail.class, entry.subclass.as_ref(), feature_type, entry.entry.level);
                out.push(FeatureTypeSlot {
                    class_id: entry.entry.class_id.clone(),
                    feature_type,
                    eligible_ids: eligible.iter().map(|f| f.id.clone()).collect(),
                    required: quota.min(eligible.len()),
                });
            }
        }
        out
    });
    // Retains only picks that are still eligible somewhere: a level/subclass
    // change can narrow a pool, and so can un-picking a Pact Boon that a
    // pact-gated invocation depended on — both are treated the same way,
    // silently dropped rather than banner'd, matching the single-class
    // precedent for pool-narrowing (as opposed to over-quota, which does get
    // a banner).
    Effect::new(move |_| {
        if !class_details_ready() || !optional_feature_pool_ready() {
            return;
        }
        let valid_ids: HashSet<String> =
            feature_slots.get().into_iter().flat_map(|slot| slot.eligible_ids).collect();
        optional_feature_choices.update(|choices| choices.retain(|id| valid_ids.contains(id)));
    });

    // Why a tab is currently locked (`None` = unlocked), reusing each step's
    // own "nothing to pick" check — shown as a disabled/grayed tab with the
    // reason as a tooltip, rather than hiding the tab outright, so its
    // existence (and why it's inapplicable right now) stays visible.
    let step_lock_reason = move |index: usize| -> Option<&'static str> {
        match index {
            2 => resolved_entries
                .get()
                .iter()
                .all(|e| e.active_types.is_empty())
                .then_some("No optional features to choose for this class/subclass yet."),
            5 => (skill_inputs.get().choice_pools.is_empty() && expertise_slot_count.get() == 0)
                .then_some("No skill or expertise choices to make yet — pick a class and background first."),
            6 => language_inputs.get().choice_pools.is_empty()
                .then_some("No language choices to make yet — pick a background and species first."),
            7 => tool_inputs.get().choice_pools.is_empty()
                .then_some("No tool choices to make yet — pick a class and background first."),
            9 => resolved_entries
                .get()
                .iter()
                .all(|e| e.spellcasting.caster_progression.is_none())
                .then_some("This build doesn't grant spellcasting."),
            10 => asi_entries.get().is_empty().then_some("No Ability Score Improvements unlocked yet."),
            _ => None,
        }
    };

    // Next/Back skip over locked tabs entirely — landing on one via these
    // buttons would be indistinguishable from a real pick, so treat them as
    // absent from the sequence rather than a stop the player passes through.
    let next_step = move |current: usize| {
        (current + 1..STEPS.len()).find(|&i| step_lock_reason(i).is_none()).unwrap_or(STEPS.len() - 1)
    };
    let prev_step =
        move |current: usize| (0..current).rev().find(|&i| step_lock_reason(i).is_none()).unwrap_or(0);

    // A compact summary of every choice made so far, for the Review step —
    // deliberately owned/`String`-based rather than threading a dozen raw
    // resources/signals into `review_step`'s parameter list.
    let review_summary = move || {
        let species = species_detail.get().and_then(|result| result.ok()).flatten();
        let background = background_detail.get().and_then(|result| result.ok()).flatten();

        // Mirrors character_sheet.rs's SheetView subtitle construction: every
        // class/subclass, joined, so a two-class build reads as e.g.
        // "Level 5 Fighter (Champion) / Level 3 Wizard".
        let mut parts: Vec<String> = resolved_entries
            .get()
            .iter()
            .map(|e| {
                let mut part = format!("Level {} {}", e.entry.level, e.detail.class.name);
                if let Some(subclass) = &e.subclass {
                    part.push_str(&format!(" ({})", subclass.short_name));
                }
                part
            })
            .collect();
        if let Some(species) = &species {
            parts.push(species.name.clone());
        }
        if let Some(background) = &background {
            parts.push(background.name.clone());
        }

        let mut skill_names =
            skill_choices.get().into_iter().flatten().map(|skill| skill_label(&skill)).collect::<Vec<_>>();
        skill_names.extend(
            custom_skill_choices
                .get()
                .into_iter()
                .flatten()
                .map(|skill| format!("{} (Custom)", skill_label(&skill))),
        );
        let expertise_names =
            expertise_choices.get().into_iter().flatten().map(|skill| skill_label(&skill)).collect::<Vec<_>>();
        let mut language_names =
            language_choices.get().into_iter().flatten().map(|language| title_case(&language)).collect::<Vec<_>>();
        language_names.extend(
            custom_language_choices
                .get()
                .into_iter()
                .flatten()
                .map(|language| format!("{} (Custom)", title_case(&language))),
        );
        let mut tool_names =
            tool_choices.get().into_iter().flatten().map(|tool| title_case(&tool)).collect::<Vec<_>>();
        tool_names.extend(
            custom_tool_choices
                .get()
                .into_iter()
                .flatten()
                .map(|tool| format!("{} (Custom)", title_case(&tool))),
        );

        let feats = feat_list.get().and_then(|result| result.ok()).unwrap_or_default();
        let asi_lines = asi_entries
            .get()
            .into_iter()
            .zip(asi_choices.get())
            .filter_map(|((_, _, asi_level), choice)| match choice {
                Some(AsiChoice::Abilities { codes }) => {
                    let increases = codes
                        .iter()
                        .map(|code| format!("+1 {}", ability_label(code)))
                        .collect::<Vec<_>>()
                        .join(", ");
                    Some(format!("Level {asi_level}: {increases}"))
                }
                Some(AsiChoice::Feat { feat_id }) => {
                    let feat_name = feats
                        .iter()
                        .find(|feat| feat.id == feat_id)
                        .map(|feat| feat.name.clone())
                        .unwrap_or_else(|| "Unknown feat".to_string());
                    Some(format!("Level {asi_level}: {feat_name}"))
                }
                None => None,
            })
            .collect::<Vec<_>>();

        let spells: Vec<Spell> =
            resolved_entries.get().iter().flat_map(|e| spell_pool_for(&e.entry.class_id)).collect();
        let cantrip_names = cantrip_choices
            .get()
            .iter()
            .filter_map(|id| spells.iter().find(|spell| &spell.id == id).map(|spell| spell.name.clone()))
            .collect::<Vec<_>>();
        let spell_names = spell_choices
            .get()
            .iter()
            .filter_map(|id| spells.iter().find(|spell| &spell.id == id).map(|spell| spell.name.clone()))
            .collect::<Vec<_>>();
        let granted_names: Vec<String> = resolved_entries
            .get()
            .iter()
            .flat_map(|e| granted_spells_for(&e.entry.class_id))
            .map(|spell| spell.name.clone())
            .collect();

        let features: Vec<OptionalFeature> =
            resolved_entries.get().iter().flat_map(|e| optional_feature_pool_for(&e.entry.class_id)).collect();
        let feature_names = optional_feature_choices
            .get()
            .iter()
            .filter_map(|id| features.iter().find(|feature| &feature.id == id).map(|feature| feature.name.clone()))
            .collect::<Vec<_>>();

        ReviewSummary {
            subtitle: parts.join(" · "),
            skill_choices: skill_names,
            expertise_choices: expertise_names,
            language_choices: language_names,
            tool_choices: tool_names,
            asi_lines,
            cantrips: cantrip_names,
            spells: spell_names,
            granted_spells: granted_names,
            optional_features: feature_names,
        }
    };

    let point_buy_spent = move || {
        ABILITY_CODES
            .iter()
            .map(|code| point_buy_cost(abilities.get().get(code)).unwrap_or(0) as u32)
            .sum::<u32>()
    };

    // Whether step `n`'s choices are complete — parameterized by index
    // (rather than reading `step.get()` directly) so the Review checklist can
    // ask about every step at once, not just whichever one is on screen.
    let step_complete = move |n: usize| match n {
        0 => !name.get().trim().is_empty(),
        1 => {
            let details = class_details_map();
            !classes.get().is_empty()
                && classes.get().iter().all(|c| {
                    details
                        .get(&c.class_id)
                        .map(|d| match subclass_unlock_level(d) {
                            Some(unlock) if unlock <= c.level => c.subclass_id.is_some(),
                            _ => true,
                        })
                        .unwrap_or(true)
                })
        }
        2 => feature_slots.get().iter().all(|slot| {
            let chosen_count = optional_feature_choices.get().iter().filter(|id| slot.eligible_ids.contains(*id)).count();
            chosen_count == slot.required
        }),
        3 => species_id.get().is_some(),
        4 => background_id.get().is_some(),
        5 => {
            skill_choices.get().len() == skill_inputs.get().choice_pools.len()
                && skill_choices.get().iter().all(Option::is_some)
                && expertise_choices.get().len() == expertise_slot_count.get()
                && expertise_choices.get().iter().all(Option::is_some)
        }
        6 => {
            language_choices.get().len() == language_inputs.get().choice_pools.len()
                && language_choices.get().iter().all(Option::is_some)
        }
        7 => {
            tool_choices.get().len() == tool_inputs.get().choice_pools.len()
                && tool_choices.get().iter().all(Option::is_some)
        }
        8 => {
            let method_ok = match ability_method.get() {
                AbilityMethod::PointBuy => point_buy_spent() <= POINT_BUY_BUDGET as u32,
                _ => true,
            };
            let ability_bonus_ok = match ability_bonus_source.get() {
                AbilityBonusSource::Species => {
                    let required_species_choices =
                        species_choose_grant().map(|(count, _, _)| count).unwrap_or(0) as usize;
                    species_ability_choices.get().len() == required_species_choices
                }
                AbilityBonusSource::Custom => {
                    let codes = custom_ability_bonus_choices.get();
                    codes.len() == 2 && codes[0] != codes[1]
                }
            };
            method_ok && ability_bonus_ok
        }
        9 => {
            cantrip_choices.get().len() == total_cantrips_required()
                && spell_choices.get().len() == total_spells_required()
                && spell_grant_choices.get().len() == spell_choice_slots.get().len()
                && spell_grant_choices.get().iter().all(Option::is_some)
        }
        10 => {
            asi_choices.get().len() == asi_entries.get().len()
                && asi_choices.get().iter().all(|choice| match choice {
                    Some(AsiChoice::Feat { feat_id }) => !feat_id.trim().is_empty(),
                    Some(AsiChoice::Abilities { codes }) => {
                        codes.len() == 2 && codes.iter().all(|code| ABILITY_CODES.contains(&code.as_str()))
                    }
                    None => false,
                })
        }
        _ => true,
    };

    let save = Action::new(|character: &Character| {
        let character = character.clone();
        async move { save_character(character).await }
    });
    let navigate = use_navigate();
    Effect::new(move |_| {
        if let Some(Ok(id)) = save.value().get() {
            navigate(&format!("/characters/{id}"), Default::default());
        }
    });

    let heading = move || {
        if character_id.get().is_empty() {
            "New Character"
        } else {
            "Edit Character"
        }
    };

    view! {
        <div class="flex flex-col gap-4">
            <a href="/characters" class="btn btn-ghost btn-sm w-fit gap-2">
                "← Back to Characters"
            </a>
            <h1 class="text-3xl">{heading}</h1>

            <div role="tablist" class="tabs tabs-box w-full text-xs flex-nowrap overflow-x-auto">
                {move || {
                    (0..STEPS.len())
                        .map(|index| {
                            let title = STEPS[index];
                            let locked = step_lock_reason(index);
                            view! {
                                <a
                                    role="tab"
                                    class=move || {
                                        let mut classes = String::from("tab");
                                        if step.get() == index {
                                            classes.push_str(" tab-active");
                                        }
                                        if locked.is_some() {
                                            classes.push_str(" opacity-40 cursor-not-allowed");
                                        } else {
                                            classes.push_str(" cursor-pointer");
                                        }
                                        classes
                                    }
                                    title=locked.unwrap_or_default()
                                    on:click=move |_| {
                                        if locked.is_none() {
                                            step.set(index);
                                        }
                                    }
                                >
                                    {title}
                                </a>
                            }
                        })
                        .collect_view()
                }}
            </div>

            <div class="card bg-base-100 border border-base-300 shadow-xl">
                <div class="card-body gap-4">
                    {move || match step.get() {
                        0 => basics_step(name, char_level, enforce_prereqs).into_any(),
                        1 => {
                            class_step(
                                    classes,
                                    class_list,
                                    class_details,
                                    current_final_abilities,
                                    enforce_prereqs,
                                )
                                .into_any()
                        }
                        2 => {
                            optional_features_step(
                                    resolved_entries,
                                    optional_feature_pool_all,
                                    feature_slots,
                                    optional_feature_choices,
                                    features_class_tab,
                                )
                                .into_any()
                        }
                        3 => {
                            species_step(
                                    species_search,
                                    species_list,
                                    species_id,
                                    species_active_group,
                                )
                                .into_any()
                        }
                        4 => {
                            background_step(background_search, background_list, background_id)
                                .into_any()
                        }
                        5 => {
                            skills_step(
                                    skill_inputs,
                                    skill_choices,
                                    expertise_slot_count,
                                    expertise_choices,
                                    skill_proficiency_sources,
                                    custom_skill_choices,
                                )
                                .into_any()
                        }
                        6 => {
                            languages_step(
                                    language_inputs,
                                    language_choices,
                                    language_proficiency_sources,
                                    custom_language_choices,
                                    all_language_names,
                                )
                                .into_any()
                        }
                        7 => {
                            tools_step(
                                    tool_inputs,
                                    tool_choices,
                                    tool_proficiency_sources,
                                    custom_tool_choices,
                                    all_tool_names,
                                )
                                .into_any()
                        }
                        8 => {
                            abilities_step(
                                    ability_method,
                                    abilities,
                                    species_detail,
                                    species_ability_choices,
                                    ability_bonus_source,
                                    custom_ability_bonus_choices,
                                )
                                .into_any()
                        }
                        9 => {
                            spells_step(
                                    resolved_entries,
                                    spell_options_all,
                                    granted_spells_all,
                                    spell_requirements,
                                    cantrip_choices,
                                    spell_choices,
                                    spells_class_tab,
                                    spell_choice_slots,
                                    spell_grant_choices,
                                )
                                .into_any()
                        }
                        10 => {
                            asi_step(
                                    resolved_entries,
                                    asi_entries,
                                    asi_choices,
                                    feat_list,
                                    asi_class_tab,
                                )
                                .into_any()
                        }
                        _ => {
                            review_step(
                                    step,
                                    step_complete,
                                    step_lock_reason,
                                    build_character,
                                    resolved_entries,
                                    species_detail,
                                    review_summary,
                                    hp_method,
                                    hp_rolls,
                                    hp_manual,
                                    save.pending(),
                                    move |character| {
                                        save.dispatch(character);
                                    },
                                )
                                .into_any()
                        }
                    }}
                    {move || {
                        save.value()
                            .get()
                            .and_then(|result| result.err())
                            .map(|err| {
                                view! {
                                    <p class="text-error text-sm">
                                        {format!("Save failed: {err}")}
                                    </p>
                                }
                            })
                    }} <div class="card-actions justify-between mt-2">
                        <button
                            type="button"
                            class="btn btn-ghost"
                            disabled=move || step.get() == 0
                            on:click=move |_| step.update(|s| *s = prev_step(*s))
                        >
                            "Back"
                        </button>
                        <button
                            type="button"
                            class=move || {
                                if step.get() == STEPS.len() - 1 {
                                    "btn btn-primary hidden"
                                } else {
                                    "btn btn-primary"
                                }
                            }
                            on:click=move |_| step.update(|s| *s = next_step(*s))
                        >
                            "Next"
                        </button>
                    </div>
                </div>
            </div>
        </div>
    }
}

#[cfg(test)]
#[path = "character_builder_tests.rs"]
mod tests;
