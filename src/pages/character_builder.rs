use crate::components::background_detail_card::BackgroundDetailCard;
use crate::components::feat_detail_card::FeatDetailCard;
use crate::components::optional_feature_detail_card::OptionalFeatureDetailCard;
use crate::components::species_detail_card::SpeciesDetailCard;
use crate::components::spell_detail_card::SpellDetailCard;
use crate::models::background::{Background, BackgroundQuery};
use crate::models::character::{
    ability_modifier, active_optional_feature_types, active_subclass, asi_levels, cantrips_known_count,
    effective_spellcasting, final_abilities, max_castable_spell_level, multiclass_active_optional_feature_types,
    multiclass_asi_slots, multiclass_class_skill_grants, multiclass_expertise_slots, multiclass_prereq_violations,
    multiclass_spell_profiles, optional_feature_quota, point_buy_cost, proficiency_bonus, resolve_ability_bonus,
    resolved_hp_max_multiclass,
    skill_slots, spells_known_count, subclass_unlock_level, total_level, AbilityBonusSource, AbilityMethod,
    AbilityScores, AsiChoice, Character, ClassLevel, HpMethod, SkillSlots, SpellcastingProfile, ABILITY_CODES,
    POINT_BUY_BUDGET, STANDARD_ARRAY,
};
use crate::models::class::{ability_label, Class, ClassDetail, Subclass};
use crate::models::feat::{Feat, FeatQuery};
use crate::models::optional_feature::{
    is_eligible, prerequisite_label, EligibilityContext, FeatureType, OptionalFeature, OptionalFeatureQuery,
    ResourceCost,
};
use crate::models::skill::{describe_skill_grants, skill_label};
use crate::models::species::{is_free_ability_choice, AbilityBonusGrant, Species, SpeciesQuery};
use crate::models::spell::Spell;
use crate::pages::backgrounds::{get_background, get_backgrounds};
use crate::pages::class_detail::get_class_detail;
use crate::pages::classes::get_classes;
use crate::pages::feats::get_feats;
use crate::pages::optional_features::get_optional_features;
use crate::pages::species::{get_species, get_species_by_id};
use crate::pages::spells::{get_class_spell_options, get_subclass_granted_spells};
use leptos::prelude::*;
use leptos::server_fn::codec::Json;
use leptos_router::hooks::{use_navigate, use_params_map};
use std::collections::{HashMap, HashSet};

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
    let class_lookup: HashMap<String, Class> =
        class_details.iter().map(|(id, detail)| (id.clone(), detail.class.clone())).collect();
    let combined_class_skills = multiclass_class_skill_grants(&character.classes, &class_lookup);
    // A choice matching a fixed grant is allowed — it's a wasted pick, not
    // an invalid one; `Character::validate()` already rejected duplicates
    // within skill_choices itself, so picking the same already-fixed skill
    // twice across slots is caught there, not here.
    let slots = skill_slots(&combined_class_skills, &background.skills);
    let skill_choices_valid = character.skill_choices.len() == slots.choice_pools.len()
        && character.skill_choices.iter().zip(&slots.choice_pools).all(|(choice, pool)| pool.contains(choice));
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
        .collect();
    let needed_expertise = multiclass_expertise_slots(&character.classes, &features_by_class);
    let expertise_valid = character.expertise_choices.len() == needed_expertise
        && character.expertise_choices.iter().all(|skill| proficient.contains(skill.as_str()));
    if !expertise_valid {
        return Err(ServerFnError::new(
            "Expertise choices don't match what's unlocked, or name a skill you aren't proficient in",
        ));
    }

    // Same reasoning again: re-derive the species' ability-bonus grant so a
    // direct POST can't persist a `species_ability_choices` pick the
    // wizard's own UI wouldn't have offered (wrong count, wrong ability, or
    // any pick at all for a species with no Choose grant) — or, in Custom
    // mode, a bonus that doesn't match the "exactly two valid abilities"
    // shape the wizard's Custom toggle offers.
    let species = crate::db::get_species(&pool, &character.species_id)
        .await
        .map_err(|err| ServerFnError::new(err.to_string()))?
        .ok_or_else(|| ServerFnError::new("Unknown species"))?;
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

/// Assigns shared spell/cantrip picks back to whichever class's own pool+budget
/// they satisfy, via Kuhn's augmenting-path algorithm — a greedy first-fit can
/// reject a valid assignment when classes' pools overlap (see `character_builder_tests.rs`).
fn attribute_multiclass_choices(chosen: &[String], pools: &[Vec<String>], required: &[usize]) -> bool {
    if chosen.len() != required.iter().sum::<usize>() {
        return false;
    }
    let mut pool_items: Vec<Vec<usize>> = vec![Vec::new(); pools.len()];
    for item in 0..chosen.len() {
        let mut visited = vec![false; pools.len()];
        if !try_assign_to_pool(item, chosen, pools, required, &mut pool_items, &mut visited) {
            return false;
        }
    }
    true
}

fn try_assign_to_pool(
    item: usize,
    chosen: &[String],
    pools: &[Vec<String>],
    required: &[usize],
    pool_items: &mut [Vec<usize>],
    visited: &mut [bool],
) -> bool {
    for i in 0..pools.len() {
        if visited[i] || !pools[i].contains(&chosen[item]) {
            continue;
        }
        visited[i] = true;
        if pool_items[i].len() < required[i] {
            pool_items[i].push(item);
            return true;
        }
        for k in 0..pool_items[i].len() {
            let displaced = pool_items[i][k];
            if try_assign_to_pool(displaced, chosen, pools, required, pool_items, visited) {
                pool_items[i][k] = item;
                return true;
            }
        }
    }
    false
}

const STEPS: [&str; 10] = [
    "Basics",
    "Class",
    "Optional Features",
    "Species",
    "Background",
    "Skills",
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
    let cantrip_choices = RwSignal::new(Vec::<String>::new());
    let spell_choices = RwSignal::new(Vec::<String>::new());
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
            cantrip_choices.set(existing.cantrip_choices);
            spell_choices.set(existing.spell_choices);
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
            cantrip_choices: cantrip_choices.get(),
            spell_choices: spell_choices.get(),
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
        let background_skills = background_detail
            .get()
            .and_then(|result| result.ok())
            .flatten()
            .map(|b| b.skills.clone())
            .unwrap_or_default();
        skill_slots(&combined_class_skills, &background_skills)
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
                if choice.as_ref().is_some_and(|skill| !pool.contains(skill) || slots.fixed.contains(skill)) {
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
        expertise_choices.update(|choices| {
            choices.resize(count, None);
            for choice in choices.iter_mut() {
                if choice.as_ref().is_some_and(|skill| !proficient.contains(skill)) {
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
            7 => resolved_entries
                .get()
                .iter()
                .all(|e| e.spellcasting.caster_progression.is_none())
                .then_some("This build doesn't grant spellcasting."),
            8 => asi_entries.get().is_empty().then_some("No Ability Score Improvements unlocked yet."),
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

        let skill_names =
            skill_choices.get().into_iter().flatten().map(|skill| skill_label(&skill)).collect::<Vec<_>>();
        let expertise_names =
            expertise_choices.get().into_iter().flatten().map(|skill| skill_label(&skill)).collect::<Vec<_>>();

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
        7 => {
            cantrip_choices.get().len() == total_cantrips_required()
                && spell_choices.get().len() == total_spells_required()
        }
        8 => {
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
                            class_step(classes, class_list, class_details, current_final_abilities, enforce_prereqs)
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
                                )
                                .into_any()
                        }
                        6 => {
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
                        7 => {
                            spells_step(
                                    resolved_entries,
                                    spell_options_all,
                                    granted_spells_all,
                                    spell_requirements,
                                    cantrip_choices,
                                    spell_choices,
                                    spells_class_tab,
                                )
                                .into_any()
                        }
                        8 => {
                            asi_step(resolved_entries, asi_entries, asi_choices, feat_list, asi_class_tab)
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

fn basics_step(name: RwSignal<String>, char_level: Memo<u8>, enforce_prereqs: RwSignal<bool>) -> impl IntoView {
    view! {
        <h2 class="card-title">"Basics"</h2>
        <label class="form-control w-full max-w-sm">
            <span class="label-text mb-1">"Character name"</span>
            <input
                type="text"
                class="input input-bordered"
                placeholder="e.g. Brenna Ironquill"
                prop:value=move || name.get()
                on:input=move |ev| name.set(event_target_value(&ev))
            />
        </label>
        <div class="flex flex-col gap-1">
            <span class="label-text">"Total level"</span>
            <span class="font-mono text-2xl">{move || format!("{:02} / 20", char_level.get())}</span>
            <span class="text-xs opacity-70">"Set on the Class step — add a class there to begin."</span>
        </div>
        <div class="flex flex-col gap-1">
            <span class="label-text">"Multiclass ability prerequisites"</span>
            <div role="tablist" class="tabs tabs-box w-fit">
                {[(true, "Enforce"), (false, "Off")]
                    .map(|(value, label)| {
                        view! {
                            <a
                                role="tab"
                                class=move || {
                                    if enforce_prereqs.get() == value { "tab tab-active" } else { "tab" }
                                }
                                on:click=move |_| enforce_prereqs.set(value)
                            >
                                {label}
                            </a>
                        }
                    })}
            </div>
            <span class="text-xs opacity-70">
                "When enforced, adding or keeping a class requires the ability scores 5e's rules call for (e.g. 13 Strength for Fighter)."
            </span>
        </div>
    }
}

fn class_step(
    classes: RwSignal<Vec<ClassLevel>>,
    class_list: Resource<Result<Vec<crate::models::class::ClassSummary>, ServerFnError>>,
    class_details: Resource<Result<(Vec<String>, Vec<(String, ClassDetail)>), ServerFnError>>,
    current_final_abilities: impl Fn() -> [(&'static str, u8); 6] + Copy + Send + Sync + 'static,
    enforce_prereqs: RwSignal<bool>,
) -> impl IntoView {
    let details_map = move || -> HashMap<String, ClassDetail> {
        class_details.get().and_then(|result| result.ok()).map(|(_, list)| list.into_iter().collect()).unwrap_or_default()
    };
    let total = move || classes.get().iter().fold(0u8, |acc, c| acc.saturating_add(c.level));
    // Soft, non-blocking hint only — Save/Review is the real gate.
    let unmet_prereq_label = move |prereqs: &[Vec<(String, u8)>], multiclass_active: bool| -> Option<String> {
        if !enforce_prereqs.get() || prereqs.is_empty() || !multiclass_active {
            return None;
        }
        let scores = current_final_abilities();
        let get = |code: &str| scores.iter().find(|(c, _)| *c == code).map(|(_, v)| *v).unwrap_or(0);
        let met = prereqs.iter().any(|group| group.iter().all(|(ability, min)| get(ability) >= *min));
        if met {
            return None;
        }
        let alternatives = prereqs
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
        Some(format!("⚠ Requires {alternatives}"))
    };

    view! {
        <h2 class="card-title">"Classes"</h2>
        <span class="font-mono text-sm opacity-80">{move || format!("TOTAL LEVEL {:02} / 20", total())}</span>

        <div class="flex flex-col gap-3">
            <For
                each=move || classes.get().into_iter().enumerate().map(|(i, c)| (c.class_id.clone(), i))
                key=|(class_id, _)| class_id.clone()
                children=move |(row_class_id, _)| {
                    let row_id = row_class_id.clone();
                    let position = move || classes.get().iter().position(|c| c.class_id == row_id).unwrap_or(0);
                    let row_id = row_class_id.clone();
                    let current_level = move || {
                        classes.get().iter().find(|c| c.class_id == row_id).map(|c| c.level).unwrap_or(1)
                    };
                    let row_id = row_class_id.clone();
                    let current_subclass_id = move || {
                        classes.get().into_iter().find(|c| c.class_id == row_id).and_then(|c| c.subclass_id)
                    };
                    let row_id = row_class_id.clone();
                    let detail = move || details_map().get(&row_id).cloned();

                    let dec_id = row_class_id.clone();
                    let dec_click = move |_: leptos::ev::MouseEvent| {
                        classes.update(|list| {
                            if let Some(entry) = list.iter_mut().find(|c| c.class_id == dec_id) {
                                entry.level = entry.level.saturating_sub(1).max(1);
                            }
                        });
                    };
                    let inc_id = row_class_id.clone();
                    let inc_click = move |_: leptos::ev::MouseEvent| {
                        classes.update(|list| {
                            if let Some(entry) = list.iter_mut().find(|c| c.class_id == inc_id) {
                                entry.level = (entry.level + 1).min(20);
                            }
                        });
                    };
                    let remove_id = row_class_id.clone();
                    let remove_click = move |_: leptos::ev::MouseEvent| {
                        classes.update(|list| list.retain(|c| c.class_id != remove_id));
                    };
                    let subclass_row_id = row_class_id.clone();

                    let detail_for_name = detail.clone();
                    let detail_for_subclass = detail.clone();
                    let detail_for_prereq_warning = detail.clone();
                    let current_level_for_dec = current_level.clone();
                    let current_level_for_display = current_level.clone();
                    let current_level_for_subclass = current_level.clone();
                    // A literal `>=` inside a `view!` attribute expression
                    // confuses the macro's tag scanner (it reads the bare `>`
                    // as closing the element early) — named boolean signals
                    // sidestep the parser entirely, same fix as the on:click
                    // handlers above.
                    let maxed_out = move || total() >= 20;

                    view! {
                        <div class=move || format!("class-band {} flex flex-col gap-2 p-3 border border-base-300 rounded-box bg-base-200/40", class_band_class(position()))>
                            <div class="flex items-center gap-3 flex-wrap">
                                <span class="font-display text-lg flex-1 min-w-40">
                                    {move || detail_for_name().map(|d| d.class.name.clone()).unwrap_or_else(|| "Loading...".to_string())}
                                </span>
                                <div class="join">
                                    <button
                                        type="button"
                                        class="btn btn-square btn-sm join-item"
                                        disabled=move || current_level_for_dec() <= 1
                                        on:click=dec_click
                                    >
                                        "−"
                                    </button>
                                    <span class="btn btn-sm join-item pointer-events-none font-mono">
                                        {move || format!("{:02}", current_level_for_display())}
                                    </span>
                                    <button
                                        type="button"
                                        class="btn btn-square btn-sm join-item"
                                        disabled=maxed_out
                                        on:click=inc_click
                                    >
                                        "+"
                                    </button>
                                </div>
                                <button
                                    type="button"
                                    class="btn btn-ghost btn-xs btn-circle"
                                    title="Remove this class"
                                    on:click=remove_click
                                >
                                    "✕"
                                </button>
                            </div>
                            {move || {
                                let detail = detail_for_prereq_warning()?;
                                let label = unmet_prereq_label(
                                    &detail.class.multiclass_ability_prerequisites,
                                    classes.get().len() >= 2,
                                )?;
                                Some(view! { <p class="text-warning text-xs">{label}</p> })
                            }}
                            {move || {
                                let detail = detail_for_subclass()?;
                                let unlock = subclass_unlock_level(&detail)?;
                                let title = detail.class.subclass_title.clone();
                                let row_id = subclass_row_id.clone();
                                Some(
                                    if unlock <= current_level_for_subclass() {
                                        view! {
                                            <div class="flex flex-col gap-2">
                                                <h3 class="font-semibold text-sm">{title}</h3>
                                                <div class="flex flex-wrap gap-2">
                                                    {detail
                                                        .subclasses
                                                        .iter()
                                                        .map(|sub| {
                                                            let sub_id = sub.subclass.id.clone();
                                                            let selected = sub.subclass.id.clone();
                                                            let label = format!(
                                                                "{} ({})",
                                                                sub.subclass.short_name,
                                                                sub.subclass.source,
                                                            );
                                                            let row_id = row_id.clone();
                                                            let click_row_id = row_id.clone();
                                                            let sub_click = move |_: leptos::ev::MouseEvent| {
                                                                classes.update(|list| {
                                                                    if let Some(entry) = list.iter_mut().find(|c| c.class_id == click_row_id) {
                                                                        entry.subclass_id = Some(sub_id.clone());
                                                                    }
                                                                });
                                                            };
                                                            let current_subclass_id = current_subclass_id.clone();
                                                            view! {
                                                                <button
                                                                    type="button"
                                                                    class=move || {
                                                                        if current_subclass_id().as_deref() == Some(selected.as_str()) {
                                                                            "btn btn-sm btn-primary"
                                                                        } else {
                                                                            "btn btn-sm btn-outline"
                                                                        }
                                                                    }
                                                                    on:click=sub_click
                                                                >
                                                                    {label}
                                                                </button>
                                                            }
                                                        })
                                                        .collect_view()}
                                                </div>
                                            </div>
                                        }
                                            .into_any()
                                    } else {
                                        view! {
                                            <p class="text-sm opacity-70">
                                                {format!("{title} unlocks at level {unlock}.")}
                                            </p>
                                        }
                                            .into_any()
                                    },
                                )
                            }}
                        </div>
                    }
                }
            />
        </div>

        <h3 class="font-semibold text-sm mt-2">"Add a class"</h3>
        <Suspense fallback=move || {
            view! { <p>"Loading classes..."</p> }
        }>
            {move || Suspend::new(async move {
                match class_list.await {
                    Ok(rows) => {
                        let added: HashSet<String> = classes.get().iter().map(|c| c.class_id.clone()).collect();
                        let maxed = total() >= 20;
                        view! {
                            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                                {rows
                                    .into_iter()
                                    .filter(|class| !added.contains(&class.id))
                                    .map(|class| {
                                        let id = class.id.clone();
                                        let saves = class
                                            .saving_throws
                                            .iter()
                                            .map(|code| ability_label(code))
                                            .collect::<Vec<_>>()
                                            .join(", ");
                                        let prereqs = class.multiclass_ability_prerequisites.clone();
                                        let warning = move || unmet_prereq_label(&prereqs, !classes.get().is_empty());
                                        view! {
                                            <button
                                                type="button"
                                                class="btn btn-outline justify-start h-auto py-2 flex-col items-start w-full"
                                                disabled=maxed
                                                on:click=move |_| {
                                                    classes.update(|list| {
                                                        list.push(ClassLevel {
                                                            class_id: id.clone(),
                                                            subclass_id: None,
                                                            level: 1,
                                                        });
                                                    });
                                                }
                                            >
                                                <span class="text-base">
                                                    {format!("{} ({})", class.name, class.source)}
                                                </span>
                                                <span class="text-xs font-normal opacity-70">
                                                    {format!("d{} · Saves: {}", class.hit_die, saves)}
                                                </span>
                                                {move || warning().map(|label| view! { <span class="text-warning text-xs">{label}</span> })}
                                            </button>
                                        }
                                    })
                                    .collect_view()}
                            </div>
                            {maxed
                                .then(|| {
                                    view! {
                                        <p class="font-mono text-xs opacity-70">
                                            "20/20 — no more classes can be added."
                                        </p>
                                    }
                                })}
                        }
                            .into_any()
                    }
                    Err(err) => {
                        view! {
                            <p class="text-error">{format!("Failed to load classes: {err}")}</p>
                        }
                            .into_any()
                    }
                }
            })}
        </Suspense>
    }
}

/// Groups species by name, preserving each name's first-occurrence order in
/// `rows` (alphabetical, since the caller's query defaults to NameAsc) —
/// collapses reprints (the same species published across multiple
/// sourcebooks) so the wizard can offer one row per name instead of one per
/// (name, source) pair.
fn group_species_by_name(rows: Vec<Species>) -> Vec<(String, Vec<Species>)> {
    let mut groups: Vec<(String, Vec<Species>)> = Vec::new();
    let mut index_by_name: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for species in rows {
        match index_by_name.get(&species.name) {
            Some(&idx) => groups[idx].1.push(species),
            None => {
                index_by_name.insert(species.name.clone(), groups.len());
                groups.push((species.name.clone(), vec![species]));
            }
        }
    }
    groups
}

fn species_subtitle(species: &Species) -> String {
    [
        species.ability.clone().unwrap_or_default(),
        species.size.clone().unwrap_or_default(),
        species.speed.clone().unwrap_or_default(),
    ]
    .into_iter()
    .filter(|part| !part.is_empty())
    .collect::<Vec<_>>()
    .join(" · ")
}

fn species_step(
    species_search: RwSignal<String>,
    species_list: Resource<Result<Vec<Species>, ServerFnError>>,
    species_id: RwSignal<Option<String>>,
    species_active_group: RwSignal<Option<String>>,
) -> impl IntoView {
    // Name of the group the currently-selected species belongs to, among the
    // currently-filtered rows — falls back to this when nothing has been
    // explicitly clicked open yet, so prefill (edit mode) expands the right
    // group with no extra wiring.
    let committed_group_name = move || -> Option<String> {
        let id = species_id.get()?;
        let rows = species_list.get()?.ok()?;
        rows.into_iter().find(|s| s.id == id).map(|s| s.name)
    };
    let active_group_name = move || species_active_group.get().or_else(committed_group_name);
    let focused_species = RwSignal::new(None::<Species>);
    // What the detail panel falls back to once the mouse leaves the list
    // entirely without a new hover/click — the committed pick, if any, or
    // nothing (back to the placeholder) — so a hover preview doesn't stick
    // around after the player moves on.
    let selected_species = move || -> Option<Species> {
        let id = species_id.get()?;
        species_list.get()?.ok()?.into_iter().find(|s| s.id == id)
    };

    view! {
        <h2 class="card-title">"Species"</h2>
        <div class="flex flex-col lg:flex-row gap-4 items-start">
            <div class="flex-1 w-full flex flex-col gap-2">
                <input
                    type="text"
                    class="input input-bordered w-full max-w-sm"
                    placeholder="Search species..."
                    prop:value=move || species_search.get()
                    on:input=move |ev| species_search.set(event_target_value(&ev))
                />
                <ul
                    class="menu bg-base-200 rounded-box max-h-[28rem] overflow-y-auto flex-nowrap w-full"
                    on:mouseleave=move |_| focused_species.set(selected_species())
                >
                    {move || match species_list.get() {
                        None => {
                            view! { <li class="p-2 text-sm opacity-60">"Loading..."</li> }.into_any()
                        }
                        Some(Err(err)) => {
                            view! {
                                <li class="p-2 text-sm text-error">
                                    {format!("Failed to load: {err}")}
                                </li>
                            }
                                .into_any()
                        }
                        Some(Ok(rows)) if rows.is_empty() => {
                            view! { <li class="p-2 text-sm opacity-60">"No matches."</li> }.into_any()
                        }
                        Some(Ok(rows)) => {
                            group_species_by_name(rows)
                                .into_iter()
                                .map(|(name, variants)| {
                                    if variants.len() <= 1 {
                                        let species = variants.into_iter().next().unwrap();
                                        let label = species.name.clone();
                                        species_row(species, label, species_id, focused_species).into_any()
                                    } else {
                                        species_group_accordion(
                                                name,
                                                variants,
                                                species_id,
                                                species_active_group,
                                                active_group_name,
                                                focused_species,
                                            )
                                            .into_any()
                                    }
                                })
                                .collect_view()
                                .into_any()
                        }
                    }}
                </ul>
            </div>
            <div class="flex-1 w-full lg:sticky lg:top-4">
                {move || match focused_species.get() {
                    Some(species) => view! { <SpeciesDetailCard species=species /> }.into_any(),
                    None => {
                        detail_panel_placeholder("Hover or select a species to see its details.")
                            .into_any()
                    }
                }}
            </div>
        </div>
    }
}

// A single species row — used both for an unambiguous (no-reprints) species
// at the top level, and for one variant nested inside a reprint group's
// accordion. `label` is caller-supplied rather than derived from
// `species.name` here, since a variant row must disambiguate by source
// ("Fake Duskkin (TBK)") while a standalone species just shows its name.
fn species_row(
    species: Species,
    label: String,
    species_id: RwSignal<Option<String>>,
    focused_species: RwSignal<Option<Species>>,
) -> impl IntoView {
    let id = species.id.clone();
    let click_id = species.id.clone();
    let is_selected = move || species_id.get().as_deref() == Some(id.as_str());
    let subtitle = species_subtitle(&species);
    let hover_species = species.clone();
    let click_species = species;

    view! {
        <li>
            <button
                type="button"
                class=move || {
                    if is_selected() {
                        "flex flex-col items-start gap-0 menu-active"
                    } else {
                        "flex flex-col items-start gap-0"
                    }
                }
                on:mouseenter=move |_| focused_species.set(Some(hover_species.clone()))
                on:click=move |_| {
                    focused_species.set(Some(click_species.clone()));
                    species_id.set(Some(click_id.clone()));
                }
            >
                <span>{label}</span>
                <span class="text-xs opacity-70">{subtitle}</span>
            </button>
        </li>
    }
}

// A species reprinted across multiple sourcebooks — rendered as an inline
// accordion row rather than a second side-by-side list: the header toggles
// `species_active_group` (a plain reactive class binding, not a native
// `<details>`, to sidestep any fight between a controlled `open` attribute
// and the browser's own toggle behavior), and its body lists each variant
// with the same hover/click wiring as a standalone species. Picking a
// variant deliberately leaves the group expanded (no reset of
// `species_active_group`) so the player can see which source they landed on.
fn species_group_accordion(
    name: String,
    variants: Vec<Species>,
    species_id: RwSignal<Option<String>>,
    species_active_group: RwSignal<Option<String>>,
    active_group_name: impl Fn() -> Option<String> + Copy + Send + Sync + 'static,
    focused_species: RwSignal<Option<Species>>,
) -> impl IntoView {
    let variant_count = variants.len();
    let variants = StoredValue::new(variants);
    let header_name_for_open = name.clone();
    let header_name_for_close = name.clone();
    let header_name_for_content = name.clone();
    let toggle_name = name.clone();
    let is_open = move || active_group_name().as_deref() == Some(header_name_for_open.as_str());
    let is_closed = move || active_group_name().as_deref() != Some(header_name_for_close.as_str());
    let is_open_for_content = move || active_group_name().as_deref() == Some(header_name_for_content.as_str());

    view! {
        <li>
            <div
                class="collapse collapse-arrow bg-base-100"
                class:collapse-open=is_open
                class:collapse-close=is_closed
            >
                <button
                    type="button"
                    class="collapse-title py-2 pl-2 pr-12 min-h-0 flex flex-row items-center justify-between w-full"
                    on:click=move |_| species_active_group.set(Some(toggle_name.clone()))
                >
                    <span>{name}</span>
                    <span class="badge badge-outline badge-sm font-normal">{variant_count}</span>
                </button>
                <div class="collapse-content p-0">
                    // Only rendered while open, not just CSS-hidden — matches
                    // the two-list version's actual absence-from-the-DOM
                    // semantics (e.g. an unopened group's variant stats
                    // shouldn't be findable by text at all, not just visually
                    // hidden by the collapse animation).
                    {move || {
                        is_open_for_content()
                            .then(|| {
                                view! {
                                    <ul class="w-full">
                                        {variants
                                            .get_value()
                                            .into_iter()
                                            .map(|species| {
                                                let label = format!("{} ({})", species.name, species.source);
                                                species_row(species, label, species_id, focused_species)
                                            })
                                            .collect_view()}
                                    </ul>
                                }
                            })
                    }}
                </div>
            </div>
        </li>
    }
}

fn background_step(
    background_search: RwSignal<String>,
    background_list: Resource<Result<Vec<Background>, ServerFnError>>,
    background_id: RwSignal<Option<String>>,
) -> impl IntoView {
    let focused_background = RwSignal::new(None::<Background>);
    // Same fallback reasoning as species_step's selected_species — the
    // detail panel reverts to the committed pick (or the placeholder, if
    // none) once the mouse leaves the list without a new hover/click.
    let selected_background = move || -> Option<Background> {
        let id = background_id.get()?;
        background_list.get()?.ok()?.into_iter().find(|b| b.id == id)
    };

    view! {
        <h2 class="card-title">"Background"</h2>
        <div class="flex flex-col lg:flex-row gap-4 items-start">
            <div class="flex-1 w-full flex flex-col gap-2">
                <input
                    type="text"
                    class="input input-bordered w-full max-w-sm"
                    placeholder="Search backgrounds..."
                    prop:value=move || background_search.get()
                    on:input=move |ev| background_search.set(event_target_value(&ev))
                />
                <ul
                    class="menu bg-base-200 rounded-box max-h-96 overflow-y-auto flex-nowrap w-full"
                    on:mouseleave=move |_| focused_background.set(selected_background())
                >
                    {move || match background_list.get() {
                        None => view! { <li class="p-2 text-sm opacity-60">"Loading..."</li> }.into_any(),
                        Some(Err(err)) => {
                            view! {
                                <li class="p-2 text-sm text-error">
                                    {format!("Failed to load: {err}")}
                                </li>
                            }
                                .into_any()
                        }
                        Some(Ok(rows)) if rows.is_empty() => {
                            view! { <li class="p-2 text-sm opacity-60">"No matches."</li> }.into_any()
                        }
                        Some(Ok(rows)) => {
                            rows.into_iter()
                                .map(|background| {
                                    let id = background.id.clone();
                                    let is_selected = move || {
                                        background_id.get().as_deref() == Some(id.as_str())
                                    };
                                    let click_id = background.id.clone();
                                    let hover_background = background.clone();
                                    let click_background = background.clone();
                                    let title = format!("{} ({})", background.name, background.source);
                                    let subtitle = if background.skills.is_empty() {
                                        "\u{2014}".to_string()
                                    } else {
                                        describe_skill_grants(&background.skills)
                                    };
                                    view! {
                                        <li>
                                            <button
                                                type="button"
                                                class=move || {
                                                    if is_selected() {
                                                        "flex flex-col items-start gap-0 menu-active"
                                                    } else {
                                                        "flex flex-col items-start gap-0"
                                                    }
                                                }
                                                on:mouseenter=move |_| {
                                                    focused_background.set(Some(hover_background.clone()))
                                                }
                                                on:click=move |_| {
                                                    focused_background.set(Some(click_background.clone()));
                                                    background_id.set(Some(click_id.clone()));
                                                }
                                            >
                                                <span>{title}</span>
                                                <span class="text-xs opacity-70">{subtitle}</span>
                                            </button>
                                        </li>
                                    }
                                })
                                .collect_view()
                                .into_any()
                        }
                    }}
                </ul>
            </div>
            <div class="flex-1 w-full lg:sticky lg:top-4">
                {move || match focused_background.get() {
                    Some(background) => view! { <BackgroundDetailCard background=background /> }.into_any(),
                    None => {
                        detail_panel_placeholder("Hover or select a background to see its details.")
                            .into_any()
                    }
                }}
            </div>
        </div>
    }
}

fn abilities_step(
    ability_method: RwSignal<AbilityMethod>,
    abilities: RwSignal<AbilityScores>,
    species_detail: Resource<Result<Option<Species>, ServerFnError>>,
    species_ability_choices: RwSignal<Vec<String>>,
    ability_bonus_source: RwSignal<AbilityBonusSource>,
    custom_ability_bonus_choices: RwSignal<Vec<String>>,
) -> impl IntoView {
    let species = move || species_detail.get().and_then(|result| result.ok()).flatten();
    // The species' one Choose grant, if it has one — same at-most-one
    // assumption as `Character::species_ability_choices`.
    let choose_grant = move || {
        species().and_then(|species| {
            species.ability_bonuses.into_iter().find_map(|grant| match grant {
                AbilityBonusGrant::Choose { count, amount, from } => Some((count, amount, from)),
                AbilityBonusGrant::Fixed { .. } => None,
            })
        })
    };
    // Base scores with the species/custom bonus folded in (but not yet ASI,
    // which isn't chosen until a later step) — the same `resolve_ability_bonus`
    // `final_abilities` uses, so this preview can never drift from the real
    // derivation.
    let bonused_scores = move || {
        resolve_ability_bonus(
            abilities.get(),
            species().as_ref(),
            ability_bonus_source.get(),
            &species_ability_choices.get(),
            &custom_ability_bonus_choices.get(),
        )
    };
    let switch_bonus_source = move |source: AbilityBonusSource| {
        if ability_bonus_source.get() == source {
            return;
        }
        ability_bonus_source.set(source);
        match source {
            AbilityBonusSource::Species => custom_ability_bonus_choices.update(Vec::clear),
            AbilityBonusSource::Custom => {
                species_ability_choices.update(Vec::clear);
                custom_ability_bonus_choices.set(vec!["str".to_string(), "dex".to_string()]);
            }
        }
    };
    let switch_method = move |method: AbilityMethod| {
        if ability_method.get() == method {
            return;
        }
        ability_method.set(method);
        abilities.set(match method {
            AbilityMethod::StandardArray => AbilityScores {
                strength: 15,
                dexterity: 14,
                constitution: 13,
                intelligence: 12,
                wisdom: 10,
                charisma: 8,
            },
            AbilityMethod::PointBuy => AbilityScores {
                strength: 8,
                dexterity: 8,
                constitution: 8,
                intelligence: 8,
                wisdom: 8,
                charisma: 8,
            },
            AbilityMethod::Manual => AbilityScores::default(),
        });
    };

    let species_hint = move || species().and_then(|species| species.ability);

    let spent = move || {
        ABILITY_CODES
            .iter()
            .map(|code| point_buy_cost(abilities.get().get(code)).unwrap_or(0) as u32)
            .sum::<u32>()
    };

    // Standard Array assignment: rather than six independent <select>s that
    // silently swap another ability's value out from under the player when
    // a score already in use is picked, the player explicitly "arms" a
    // score from this pool, then clicks the ability to drop it into — the
    // same underlying swap, but driven by two deliberate clicks instead of
    // a surprising side effect of one dropdown.
    let armed_value = RwSignal::new(None::<u8>);

    view! {
        <h2 class="card-title">"Ability Scores"</h2>
        <div role="tablist" class="tabs tabs-box w-fit">
            {[
                (AbilityMethod::StandardArray, "Standard Array"),
                (AbilityMethod::PointBuy, "Point Buy"),
                (AbilityMethod::Manual, "Manual"),
            ]
                .map(|(method, label)| {
                    view! {
                        <a
                            role="tab"
                            class=move || {
                                if ability_method.get() == method {
                                    "tab tab-active"
                                } else {
                                    "tab"
                                }
                            }
                            on:click=move |_| switch_method(method)
                        >
                            {label}
                        </a>
                    }
                })}
        </div>

        <div role="tablist" class="tabs tabs-box w-fit">
            {[
                (AbilityBonusSource::Species, "Species bonus"),
                (AbilityBonusSource::Custom, "Custom bonus"),
            ]
                .map(|(source, label)| {
                    view! {
                        <a
                            role="tab"
                            class=move || {
                                if ability_bonus_source.get() == source {
                                    "tab tab-active"
                                } else {
                                    "tab"
                                }
                            }
                            on:click=move |_| switch_bonus_source(source)
                        >
                            {label}
                        </a>
                    }
                })}
        </div>

        {move || {
            if ability_bonus_source.get() != AbilityBonusSource::Species {
                return None;
            }
            species_hint()
                .map(|hint| {
                    view! { <p class="text-sm opacity-70">{format!("Species bonus: {hint}")}</p> }
                })
        }}

        {move || {
            if ability_bonus_source.get() != AbilityBonusSource::Species {
                return None;
            }
            choose_grant()
                .map(|(count, amount, from)| {
                    let candidates: Vec<&'static str> = ABILITY_CODES
                        .into_iter()
                        .filter(|code| {
                            is_free_ability_choice(&from) || from.contains(&code.to_string())
                        })
                        .collect();
                    view! {
                        <div class="flex flex-col gap-1">
                            <p class="text-sm opacity-70">
                                {format!("Choose {count} to gain {amount:+} each:")}
                            </p>
                            <div class="flex flex-wrap gap-2">
                                {candidates
                                    .into_iter()
                                    .map(|code| {
                                        let selected = move || {
                                            species_ability_choices
                                                .get()
                                                .iter()
                                                .any(|picked| picked == code)
                                        };
                                        view! {
                                            <button
                                                type="button"
                                                class=move || {
                                                    if selected() {
                                                        "btn btn-sm btn-primary"
                                                    } else {
                                                        "btn btn-sm btn-outline"
                                                    }
                                                }
                                                disabled=move || {
                                                    !selected()
                                                        && species_ability_choices.get().len() >= count as usize
                                                }
                                                on:click=move |_| {
                                                    species_ability_choices
                                                        .update(|choices| {
                                                            if let Some(pos) = choices
                                                                .iter()
                                                                .position(|picked| picked == code)
                                                            {
                                                                choices.remove(pos);
                                                            } else {
                                                                choices.push(code.to_string());
                                                            }
                                                        });
                                                }
                                            >
                                                {ability_label(code)}
                                            </button>
                                        }
                                    })
                                    .collect_view()}
                            </div>
                        </div>
                    }
                })
        }}

        {move || {
            if ability_bonus_source.get() != AbilityBonusSource::Custom {
                return None;
            }
            // Disabled once picked in the other slot — a repeated code isn't
            // meaningful here, unlike `AsiChoice::Abilities`'s "same twice
            // for +2" (see `apply_custom_species_bonus`'s doc comment).
            let is_disabled = move |position: usize, code: &'static str| {
                custom_ability_bonus_choices
                    .get()
                    .iter()
                    .enumerate()
                    .any(|(i, picked)| i != position && picked == code)
            };
            let custom_ability_select = move |position: usize, label: &'static str| {
                let codes = custom_ability_bonus_choices.get();
                let current = codes.get(position).cloned().unwrap_or_else(|| "str".to_string());
                view! {
                    <div class="flex flex-col gap-1">
                        <span class="text-xs opacity-70">{label}</span>
                        <select
                            class="select select-bordered select-sm"
                            on:change=move |ev| {
                                custom_ability_bonus_choices
                                    .update(|codes| {
                                        codes.resize(2, "str".to_string());
                                        codes[position] = event_target_value(&ev);
                                    });
                            }
                        >
                            {ABILITY_CODES
                                .map(|code| {
                                    let selected = current.clone();
                                    view! {
                                        <option
                                            value=code
                                            selected=move || selected == code
                                            disabled=move || is_disabled(position, code)
                                        >
                                            {ability_label(code)}
                                        </option>
                                    }
                                })}
                        </select>
                    </div>
                }
            };
            Some(
                view! {
                    <div class="flex flex-col gap-1">
                        <p class="text-sm opacity-70">
                            "Pick two different abilities: the first gains +2, the second +1."
                        </p>
                        <div class="flex flex-wrap gap-2 items-end">
                            {custom_ability_select(0, "+2")} {custom_ability_select(1, "+1")}
                        </div>
                    </div>
                },
            )
        }}

        {move || match ability_method.get() {
            AbilityMethod::StandardArray => {
                view! {
                    <div class="flex flex-col gap-2">
                        <p class="text-sm opacity-70">
                            "Click a score, then click an ability to assign it."
                        </p>
                        <div class="flex flex-wrap gap-2">
                            {STANDARD_ARRAY
                                .map(|value| {
                                    let owner = move || {
                                        ABILITY_CODES
                                            .iter()
                                            .find(|code| abilities.get().get(code) == value)
                                            .map(|code| code.to_uppercase())
                                            .unwrap_or_default()
                                    };
                                    view! {
                                        <button
                                            type="button"
                                            class=move || {
                                                if armed_value.get() == Some(value) {
                                                    "btn btn-sm btn-primary h-auto py-1 flex-col gap-0"
                                                } else {
                                                    "btn btn-sm btn-outline h-auto py-1 flex-col gap-0"
                                                }
                                            }
                                            on:click=move |_| {
                                                armed_value
                                                    .update(|armed| {
                                                        *armed = if *armed == Some(value) {
                                                            None
                                                        } else {
                                                            Some(value)
                                                        };
                                                    })
                                            }
                                        >
                                            <span class="text-base font-semibold">
                                                {value.to_string()}
                                            </span>
                                            <span class="text-[10px] font-normal opacity-70">
                                                {owner}
                                            </span>
                                        </button>
                                    }
                                })}
                        </div>
                    </div>
                }
                    .into_any()
            }
            AbilityMethod::PointBuy => {
                view! {
                    <p class="text-sm">
                        {move || format!("{} of {POINT_BUY_BUDGET} points spent", spent())}
                    </p>
                }
                    .into_any()
            }
            _ => ().into_any(),
        }}

        <div class="grid grid-cols-2 sm:grid-cols-3 gap-3 max-w-xl">
            {ABILITY_CODES
                .map(|code| {
                    let score = move || abilities.get().get(code);
                    let bonus = move || bonused_scores().get(code) as i16 - score() as i16;
                    let modifier = move || {
                        format!("{:+}", ability_modifier(bonused_scores().get(code)))
                    };
                    view! {
                        <div class="flex flex-col gap-1 p-2 border border-base-300 rounded-box">
                            <span class="text-sm font-semibold">
                                {ability_label(code)} " " <span class="opacity-60">{modifier}</span>
                            </span>
                            {move || match ability_method.get() {
                                AbilityMethod::StandardArray => {
                                    view! {
                                        <button
                                            type="button"
                                            class="btn btn-outline btn-sm w-full text-lg font-bold"
                                            disabled=move || {
                                                armed_value.get().is_none_or(|value| value == score())
                                            }
                                            on:click=move |_| {
                                                if let Some(value) = armed_value.get() {
                                                    abilities
                                                        .update(|scores| {
                                                            let current = scores.get(code);
                                                            if let Some(other) = ABILITY_CODES
                                                                .iter()
                                                                .find(|other| scores.get(other) == value)
                                                            {
                                                                scores.set(other, current);
                                                            }
                                                            scores.set(code, value);
                                                        });
                                                    armed_value.set(None);
                                                }
                                            }
                                        >
                                            {move || score().to_string()}
                                        </button>
                                    }
                                        .into_any()
                                }
                                AbilityMethod::PointBuy => {
                                    view! {
                                        <div class="join">
                                            <button
                                                type="button"
                                                class="btn btn-sm join-item"
                                                disabled=move || abilities.get().get(code) <= 8
                                                on:click=move |_| {
                                                    abilities
                                                        .update(|scores| {
                                                            scores.set(code, scores.get(code).saturating_sub(1).max(8))
                                                        })
                                                }
                                            >
                                                "-"
                                            </button>
                                            <span class="btn btn-sm join-item pointer-events-none">
                                                {move || score().to_string()}
                                            </span>
                                            <button
                                                type="button"
                                                class="btn btn-sm join-item"
                                                disabled=move || {
                                                    let scores = abilities.get();
                                                    let next = scores.get(code) + 1;
                                                    match point_buy_cost(next) {
                                                        None => true,
                                                        Some(next_cost) => {
                                                            let current_cost = point_buy_cost(scores.get(code))
                                                                .unwrap_or(0);
                                                            spent() + (next_cost - current_cost) as u32
                                                                > POINT_BUY_BUDGET as u32
                                                        }
                                                    }
                                                }
                                                on:click=move |_| {
                                                    abilities
                                                        .update(|scores| scores.set(code, scores.get(code) + 1))
                                                }
                                            >
                                                "+"
                                            </button>
                                        </div>
                                    }
                                        .into_any()
                                }
                                AbilityMethod::Manual => {
                                    view! {
                                        <input
                                            type="number"
                                            class="input input-bordered input-sm"
                                            min="3"
                                            max="20"
                                            prop:value=move || score().to_string()
                                            on:input=move |ev| {
                                                if let Ok(value) = event_target_value(&ev).parse::<u8>() {
                                                    abilities
                                                        .update(|scores| scores.set(code, value.clamp(3, 20)));
                                                }
                                            }
                                        />
                                    }
                                        .into_any()
                                }
                            }}
                            {move || {
                                let bonus = bonus();
                                (bonus != 0)
                                    .then(|| {
                                        view! {
                                            <span class="text-xs opacity-70">
                                                {format!("{bonus:+} species")}
                                            </span>
                                        }
                                    })
                            }}
                        </div>
                    }
                })}
        </div>
    }
}

fn asi_step(
    resolved_entries: Memo<Vec<ResolvedClassEntry>>,
    asi_entries: Memo<Vec<(usize, String, u8)>>,
    asi_choices: RwSignal<Vec<Option<AsiChoice>>>,
    feat_list: Resource<Result<Vec<crate::models::feat::Feat>, ServerFnError>>,
    asi_class_tab: RwSignal<usize>,
) -> impl IntoView {
    view! {
        <h2 class="card-title">"Feats & Ability Score Improvements"</h2>
        {move || {
            let entries = resolved_entries.get();
            // Global slot position (indexes the flat `asi_choices` vec)
            // alongside which resolved entry granted it and at what level.
            let indexed_slots: Vec<(usize, usize, u8)> = asi_entries
                .get()
                .into_iter()
                .enumerate()
                .map(|(slot, (entry_index, _, slot_level))| (slot, entry_index, slot_level))
                .collect();
            if indexed_slots.is_empty() {
                return view! {
                    <p class="text-sm opacity-70">
                        "No improvement slots at this level — most classes unlock their first at level 4."
                    </p>
                }
                    .into_any();
            }
            // Only tab classes that actually have an unlocked slot.
            let tab_entries: Vec<ResolvedClassEntry> = entries
                .into_iter()
                .filter(|e| indexed_slots.iter().any(|(_, entry_index, _)| *entry_index == e.index))
                .collect();
            if tab_entries.len() <= 1 {
                return indexed_slots
                    .into_iter()
                    .map(|(slot, _, slot_level)| asi_slot(slot, slot_level, asi_choices, feat_list))
                    .collect_view()
                    .into_any();
            }
            let active = asi_class_tab.get().min(tab_entries.len().saturating_sub(1));
            let active_entry = &tab_entries[active];
            view! {
                <div role="tablist" class="tabs tabs-boxed tabs-sm w-fit">
                    {tab_entries
                        .iter()
                        .enumerate()
                        .map(|(tab_index, e)| {
                            let is_active = tab_index == active;
                            let label = e.detail.class.name.clone();
                            view! {
                                <a
                                    role="tab"
                                    class=format!(
                                        "tab {} {}",
                                        class_band_class(e.index),
                                        if is_active { "tab-active class-tab-active" } else { "" },
                                    )
                                    on:click=move |_| asi_class_tab.set(tab_index)
                                >
                                    {label}
                                </a>
                            }
                        })
                        .collect_view()}
                </div>
                <div class="flex flex-col gap-2">
                    {indexed_slots
                        .into_iter()
                        .filter(|(_, entry_index, _)| *entry_index == active_entry.index)
                        .map(|(slot, _, slot_level)| asi_slot(
                            slot,
                            slot_level,
                            asi_choices,
                            feat_list,
                        ))
                        .collect_view()}
                </div>
            }
                .into_any()
        }}
    }
}

fn asi_slot(
    slot: usize,
    slot_level: u8,
    asi_choices: RwSignal<Vec<Option<AsiChoice>>>,
    feat_list: Resource<Result<Vec<crate::models::feat::Feat>, ServerFnError>>,
) -> impl IntoView {
    let choice = move || asi_choices.get().get(slot).cloned().flatten();
    let set_choice = move |value: Option<AsiChoice>| {
        asi_choices.update(|choices| {
            if let Some(entry) = choices.get_mut(slot) {
                *entry = value;
            }
        });
    };
    let focused_feat = RwSignal::new(None::<Feat>);

    let kind = move || match choice() {
        None => "none",
        Some(AsiChoice::Abilities { .. }) => "abilities",
        Some(AsiChoice::Feat { .. }) => "feat",
    };

    let ability_select = move |position: usize| {
        let codes = match choice() {
            Some(AsiChoice::Abilities { codes }) => codes,
            _ => vec!["str".to_string(), "str".to_string()],
        };
        let current = codes.get(position).cloned().unwrap_or_else(|| "str".to_string());
        view! {
            <select
                class="select select-bordered select-sm"
                on:change=move |ev| {
                    let mut codes = match choice() {
                        Some(AsiChoice::Abilities { codes }) => codes,
                        _ => vec!["str".to_string(), "str".to_string()],
                    };
                    codes.resize(2, "str".to_string());
                    codes[position] = event_target_value(&ev);
                    set_choice(Some(AsiChoice::Abilities { codes }));
                }
            >
                {ABILITY_CODES
                    .map(|code| {
                        let selected = current.clone();
                        view! {
                            <option value=code selected=move || selected == code>
                                {ability_label(code)}
                            </option>
                        }
                    })}
            </select>
        }
    };

    view! {
        <div class="flex flex-col gap-2 p-3 border border-base-300 rounded-box">
            <span class="font-semibold text-sm">{format!("Level {slot_level} improvement")}</span>
            <div class="flex flex-wrap gap-2 items-center">
                <select
                    class="select select-bordered select-sm w-40"
                    on:change=move |ev| {
                        match event_target_value(&ev).as_str() {
                            "abilities" => {
                                set_choice(
                                    Some(AsiChoice::Abilities {
                                        codes: vec!["str".to_string(), "str".to_string()],
                                    }),
                                )
                            }
                            "feat" => {
                                set_choice(
                                    Some(AsiChoice::Feat {
                                        feat_id: String::new(),
                                    }),
                                )
                            }
                            _ => set_choice(None),
                        }
                    }
                >
                    <option value="none" selected=move || kind() == "none">
                        "— unspent —"
                    </option>
                    <option value="abilities" selected=move || kind() == "abilities">
                        "Ability increases"
                    </option>
                    <option value="feat" selected=move || kind() == "feat">
                        "Feat"
                    </option>
                </select>

                {move || match choice() {
                    Some(AsiChoice::Abilities { .. }) => {
                        view! {
                            <span class="text-sm opacity-70">
                                "+1 to each (same twice for +2):"
                            </span>
                            {ability_select(0)}
                            {ability_select(1)}
                        }
                            .into_any()
                    }
                    _ => ().into_any(),
                }}
            </div>
            {move || match choice() {
                Some(AsiChoice::Feat { feat_id }) => {
                    view! {
                        <Suspense fallback=move || {
                            view! { <span class="text-sm">"Loading feats..."</span> }
                        }>
                            {move || {
                                let current = feat_id.clone();
                                feat_list
                                    .get()
                                    .map(|result| match result {
                                        Ok(feats) => {
                                            let committed_feat = {
                                                let current = current.clone();
                                                let feats = feats.clone();
                                                move || -> Option<Feat> {
                                                    feats.iter().find(|f| f.id == current).cloned()
                                                }
                                            };
                                            Effect::new({
                                                let committed_feat = committed_feat.clone();
                                                move |_| focused_feat.set(committed_feat())
                                            });
                                            view! {
                                                <div class="flex flex-col lg:flex-row gap-3 w-full">
                                                    <ul
                                                        class="menu menu-sm bg-base-200 rounded-box max-h-72 overflow-y-auto flex-nowrap flex-1 min-w-0"
                                                        on:mouseleave=move |_| {
                                                            focused_feat.set(committed_feat())
                                                        }
                                                    >
                                                        {feats
                                                            .into_iter()
                                                            .map(|feat| {
                                                                asi_feat_row(feat, slot, asi_choices, focused_feat)
                                                            })
                                                            .collect_view()}
                                                    </ul>
                                                    <div class="flex-1 w-full">
                                                        {move || match focused_feat.get() {
                                                            Some(feat) => {
                                                                view! { <FeatDetailCard feat=feat /> }.into_any()
                                                            }
                                                            None => {
                                                                detail_panel_placeholder(
                                                                        "Hover or select a feat to see its details.",
                                                                    )
                                                                    .into_any()
                                                            }
                                                        }}
                                                    </div>
                                                </div>
                                            }
                                                .into_any()
                                        }
                                        Err(err) => {
                                            view! {
                                                <span class="text-sm text-error">
                                                    {format!("Failed to load feats: {err}")}
                                                </span>
                                            }
                                                .into_any()
                                        }
                                    })
                            }}
                        </Suspense>
                    }
                        .into_any()
                }
                _ => ().into_any(),
            }}
        </div>
    }
}

// A single feat row inside an ASI slot's "Feat" choice — mirrors
// `species_row`'s hover-to-preview/click-to-select pattern, but scoped to
// one slot's own `focused_feat` signal so multiple slots don't fight over
// which feat is shown.
fn asi_feat_row(
    feat: Feat,
    slot: usize,
    asi_choices: RwSignal<Vec<Option<AsiChoice>>>,
    focused_feat: RwSignal<Option<Feat>>,
) -> impl IntoView {
    let id = feat.id.clone();
    let click_id = feat.id.clone();
    let is_selected = move || {
        matches!(
            asi_choices.get().get(slot).cloned().flatten(),
            Some(AsiChoice::Feat { feat_id }) if feat_id == id
        )
    };
    let label = feat.name.clone();
    let subtitle = match &feat.prerequisite {
        Some(prereq) => format!("{} — {}", feat.source, prereq),
        None => feat.source.clone(),
    };
    let hover_feat = feat.clone();
    let click_feat = feat;

    view! {
        <li>
            <button
                type="button"
                class=move || {
                    if is_selected() {
                        "flex flex-col items-start gap-0 menu-active"
                    } else {
                        "flex flex-col items-start gap-0"
                    }
                }
                on:mouseenter=move |_| focused_feat.set(Some(hover_feat.clone()))
                on:click=move |_| {
                    focused_feat.set(Some(click_feat.clone()));
                    asi_choices
                        .update(|choices| {
                            if let Some(entry) = choices.get_mut(slot) {
                                *entry = Some(AsiChoice::Feat {
                                    feat_id: click_id.clone(),
                                });
                            }
                        });
                }
            >
                <span>{label}</span>
                <span class="text-xs opacity-70">{subtitle}</span>
            </button>
        </li>
    }
}

fn skills_step(
    skill_inputs: Memo<SkillSlots>,
    skill_choices: RwSignal<Vec<Option<String>>>,
    expertise_slot_count: Memo<usize>,
    expertise_choices: RwSignal<Vec<Option<String>>>,
) -> impl IntoView {
    view! {
        <h2 class="card-title">"Skill Proficiencies"</h2>
        {move || {
            (skill_inputs.get().choice_pools.is_empty())
                .then(|| {
                    view! {
                        <p class="text-sm opacity-70">
                            "No skill choices to make — this class and background only grant fixed proficiencies."
                        </p>
                    }
                })
        }}
        // Keyed on slot index so a `skill_inputs` recompute that leaves the
        // slot count/content unchanged (e.g. once a still-loading resource
        // resolves) patches in place instead of tearing down and rebuilding
        // every <select>, which could otherwise race with an in-flight
        // selection and silently drop it.
        <For
            each=move || skill_inputs.get().choice_pools.into_iter().enumerate()
            key=|(slot, _)| *slot
            children=move |(slot, pool)| skill_slot(slot, pool, skill_inputs, skill_choices)
        />
        {move || {
            (expertise_slot_count.get() > 0)
                .then(|| view! { <h2 class="card-title mt-4">"Expertise"</h2> })
        }}
        <For
            each=move || 0..expertise_slot_count.get()
            key=|slot| *slot
            children=move |slot| expertise_slot(
                slot,
                skill_inputs,
                skill_choices,
                expertise_choices,
            )
        />
    }
}

fn skill_slot(
    slot: usize,
    pool: Vec<String>,
    skill_inputs: Memo<SkillSlots>,
    skill_choices: RwSignal<Vec<Option<String>>>,
) -> impl IntoView {
    let choice = move || skill_choices.get().get(slot).cloned().flatten();
    let set_choice = move |value: Option<String>| {
        skill_choices.update(|choices| {
            if let Some(entry) = choices.get_mut(slot) {
                *entry = value;
            }
        });
    };
    // Disabled once picked in a sibling slot, or once already granted
    // outright by class/background — picking it here would just waste the
    // slot on a proficiency the character already has.
    let is_disabled = move |opt: &str| {
        skill_inputs.get().fixed.iter().any(|fixed| fixed == opt)
            || skill_choices
                .get()
                .iter()
                .enumerate()
                .any(|(i, picked)| i != slot && picked.as_deref() == Some(opt))
    };

    view! {
        <div class="flex flex-col gap-1 p-3 border border-base-300 rounded-box">
            <span class="font-semibold text-sm">{format!("Skill choice {}", slot + 1)}</span>
            <select
                class="select select-bordered select-sm w-64"
                on:change=move |ev| {
                    let value = event_target_value(&ev);
                    set_choice(if value.is_empty() { None } else { Some(value) });
                }
            >
                <option value="" selected=move || choice().is_none()>
                    "— choose —"
                </option>
                {pool
                    .into_iter()
                    .map(|opt| {
                        let value = opt.clone();
                        let label = skill_label(&opt);
                        let selected_opt = opt.clone();
                        let disabled_opt = opt.clone();
                        view! {
                            <option
                                value=value
                                selected=move || choice().as_deref() == Some(selected_opt.as_str())
                                disabled=move || is_disabled(&disabled_opt)
                            >
                                {label}
                            </option>
                        }
                    })
                    .collect_view()}
            </select>
        </div>
    }
}

fn expertise_slot(
    slot: usize,
    skill_inputs: Memo<SkillSlots>,
    skill_choices: RwSignal<Vec<Option<String>>>,
    expertise_choices: RwSignal<Vec<Option<String>>>,
) -> impl IntoView {
    let choice = move || expertise_choices.get().get(slot).cloned().flatten();
    let set_choice = move |value: Option<String>| {
        expertise_choices.update(|choices| {
            if let Some(entry) = choices.get_mut(slot) {
                *entry = value;
            }
        });
    };
    let proficient_pool = move || {
        let mut pool = skill_inputs.get().fixed;
        pool.extend(skill_choices.get().into_iter().flatten());
        pool.sort();
        pool.dedup();
        pool
    };
    let is_disabled = move |opt: &str| {
        expertise_choices
            .get()
            .iter()
            .enumerate()
            .any(|(i, picked)| i != slot && picked.as_deref() == Some(opt))
    };

    view! {
        <div class="flex flex-col gap-1 p-3 border border-base-300 rounded-box">
            <span class="font-semibold text-sm">{format!("Expertise choice {}", slot + 1)}</span>
            <select
                class="select select-bordered select-sm w-64"
                on:change=move |ev| {
                    let value = event_target_value(&ev);
                    set_choice(if value.is_empty() { None } else { Some(value) });
                }
            >
                <option value="" selected=move || choice().is_none()>
                    "— choose —"
                </option>
                {move || {
                    proficient_pool()
                        .into_iter()
                        .map(|opt| {
                            let value = opt.clone();
                            let label = skill_label(&opt);
                            let selected_opt = opt.clone();
                            let disabled_opt = opt.clone();
                            view! {
                                <option
                                    value=value
                                    selected=move || {
                                        choice().as_deref() == Some(selected_opt.as_str())
                                    }
                                    disabled=move || is_disabled(&disabled_opt)
                                >
                                    {label}
                                </option>
                            }
                        })
                        .collect_view()
                }}
            </select>
        </div>
    }
}

fn spells_step(
    resolved_entries: Memo<Vec<ResolvedClassEntry>>,
    spell_options_all: Resource<Result<(Vec<EntryKey>, Vec<(String, Vec<Spell>)>), ServerFnError>>,
    granted_spells_all: Resource<Result<Vec<(String, Vec<Spell>)>, ServerFnError>>,
    spell_requirements: Memo<Vec<SpellRequirement>>,
    cantrip_choices: RwSignal<Vec<String>>,
    spell_choices: RwSignal<Vec<String>>,
    spells_class_tab: RwSignal<usize>,
) -> impl IntoView {
    // Local to this step (not lifted to CharacterBuilderPage) — same "focused
    // detail" role `selected_spell` plays on the compendium Spells page, just
    // driven by hover as well as click so a mouse user can preview without
    // committing a pick.
    let focused_spell = RwSignal::new(None::<Spell>);

    // Non-caster classes simply have no sub-tab here.
    let caster_entries = Memo::new(move |_| -> Vec<ResolvedClassEntry> {
        resolved_entries.get().into_iter().filter(|e| e.spellcasting.caster_progression.is_some()).collect()
    });
    let active_index =
        Memo::new(move |_| spells_class_tab.get().min(caster_entries.get().len().saturating_sub(1)));
    let active_class_id =
        Memo::new(move |_| caster_entries.get().get(active_index.get()).map(|e| e.entry.class_id.clone()));
    let cantrips_required = Memo::new(move |_| {
        active_class_id
            .get()
            .and_then(|id| spell_requirements.get().into_iter().find(|r| r.class_id == id).map(|r| r.cantrips_required))
            .unwrap_or(0)
    });
    let spells_required = Memo::new(move |_| {
        active_class_id
            .get()
            .and_then(|id| spell_requirements.get().into_iter().find(|r| r.class_id == id).map(|r| r.spells_required))
            .unwrap_or(0)
    });

    view! {
        <h2 class="card-title">"Spells"</h2>
        {move || {
            caster_entries
                .get()
                .is_empty()
                .then(|| {
                    view! {
                        <p class="text-sm opacity-70">"This build doesn't grant spellcasting."</p>
                    }
                })
        }}
        {move || {
            (caster_entries.get().len() > 1)
                .then(|| {
                    view! {
                        <div role="tablist" class="tabs tabs-boxed tabs-sm w-fit">
                            {caster_entries
                                .get()
                                .iter()
                                .enumerate()
                                .map(|(tab_index, e)| {
                                    let is_active = tab_index == active_index.get();
                                    let label = e.detail.class.name.clone();
                                    view! {
                                        <a
                                            role="tab"
                                            class=format!(
                                                "tab {} {}",
                                                class_band_class(e.index),
                                                if is_active { "tab-active class-tab-active" } else { "" },
                                            )
                                            on:click=move |_| spells_class_tab.set(tab_index)
                                        >
                                            {label}
                                        </a>
                                    }
                                })
                                .collect_view()}
                        </div>
                    }
                })
        }}
        <div class="flex flex-col lg:flex-row gap-4 items-start">
            <div class="flex-1 w-full flex flex-col gap-4">
                <Suspense fallback=move || ()>
                    {move || Suspend::new(async move {
                        let Some(class_id) = active_class_id.get() else { return ().into_any() };
                        match granted_spells_all.await {
                            Ok(list) => {
                                let spells =
                                    list.into_iter().find(|(id, _)| *id == class_id).map(|(_, s)| s).unwrap_or_default();
                                if spells.is_empty() {
                                    return ().into_any();
                                }
                                let names = spells
                                    .iter()
                                    .map(|spell| spell.name.clone())
                                    .collect::<Vec<_>>()
                                    .join(", ");
                                view! {
                                    <div role="alert" class="alert alert-info alert-soft text-sm py-2">
                                        <span>
                                            {format!(
                                                "Your subclass automatically gives you these — always prepared, not shown below: {names}",
                                            )}
                                        </span>
                                    </div>
                                }
                                    .into_any()
                            }
                            Err(_) => ().into_any(),
                        }
                    })}
                </Suspense>
                <Suspense fallback=move || {
                    view! { <p>"Loading spells..."</p> }
                }>
                    {move || Suspend::new(async move {
                        let Some(class_id) = active_class_id.get() else { return ().into_any() };
                        match spell_options_all.await {
                            Ok((_, list)) => {
                                let spells =
                                    list.into_iter().find(|(id, _)| *id == class_id).map(|(_, s)| s).unwrap_or_default();
                                let cantrips: Vec<Spell> = spells
                                    .iter()
                                    .filter(|spell| spell.level == 0)
                                    .cloned()
                                    .collect();
                                let leveled: Vec<Spell> = spells
                                    .iter()
                                    .filter(|spell| spell.level > 0)
                                    .cloned()
                                    .collect();
                                view! {
                                    {(cantrips_required.get() > 0)
                                        .then(|| {
                                            spell_chip_section(
                                                "Cantrips",
                                                cantrips,
                                                cantrip_choices,
                                                cantrips_required,
                                                focused_spell,
                                            )
                                        })}
                                    {(spells_required.get() > 0)
                                        .then(|| {
                                            leveled_spell_chip_section(
                                                leveled,
                                                spell_choices,
                                                spells_required,
                                                focused_spell,
                                            )
                                        })}
                                }
                                    .into_any()
                            }
                            Err(err) => {
                                view! {
                                    <p class="text-error">{format!("Failed to load spells: {err}")}</p>
                                }
                                    .into_any()
                            }
                        }
                    })}
                </Suspense>
            </div>
            <div class="w-full lg:w-96 lg:sticky lg:top-4">
                {move || match focused_spell.get() {
                    Some(spell) => view! { <SpellDetailCard spell=spell /> }.into_any(),
                    None => {
                        detail_panel_placeholder("Hover or select a spell to see its details.").into_any()
                    }
                }}
            </div>
        </div>
    }
}

// Shared empty state for the Background/Species/Spells steps' right-side
// detail panel, before anything has been hovered or selected.
fn detail_panel_placeholder(message: &'static str) -> impl IntoView {
    view! {
        <div class="card bg-base-200 border border-base-300 border-dashed">
            <div class="card-body items-center text-center py-8">
                <p class="text-sm opacity-60">{message}</p>
            </div>
        </div>
    }
}

// Display order for Optional Features sections — mostly cosmetic, but Pact
// Boon is deliberately placed right before Eldritch Invocation (rather than
// alphabetical, which would reverse them) so a Warlock sees "pick your pact"
// on the left and "pick your invocations" (some of which need that pact) on
// the right, in the 2-column grid below.
const OPTIONAL_FEATURE_DISPLAY_ORDER: [FeatureType; 12] = [
    FeatureType::PactBoon,
    FeatureType::EldritchInvocation,
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

type OptionalFeaturePoolResource = Resource<Result<(Vec<EntryKey>, Vec<(String, Vec<OptionalFeature>)>), ServerFnError>>;

// One sub-tab per class that currently grants any optional features (only shown once there's more than one).
fn optional_features_step(
    resolved_entries: Memo<Vec<ResolvedClassEntry>>,
    optional_feature_pool_all: OptionalFeaturePoolResource,
    feature_slots: Memo<Vec<FeatureTypeSlot>>,
    choices: RwSignal<Vec<String>>,
    features_class_tab: RwSignal<usize>,
) -> impl IntoView {
    let entries_with_types = Memo::new(move |_| -> Vec<ResolvedClassEntry> {
        resolved_entries.get().into_iter().filter(|e| !e.active_types.is_empty()).collect()
    });
    let active_index =
        Memo::new(move |_| features_class_tab.get().min(entries_with_types.get().len().saturating_sub(1)));
    let active_entry = Memo::new(move |_| entries_with_types.get().get(active_index.get()).cloned());
    let focused_feature = RwSignal::new(None::<OptionalFeature>);

    view! {
        <h2 class="card-title">"Optional Features"</h2>
        {move || {
            entries_with_types
                .get()
                .is_empty()
                .then(|| {
                    view! {
                        <p class="text-sm opacity-70">
                            "No optional features to choose — no class here grants any at this level."
                        </p>
                    }
                })
        }}
        {move || {
            (entries_with_types.get().len() > 1)
                .then(|| {
                    view! {
                        <div role="tablist" class="tabs tabs-boxed tabs-sm w-fit">
                            {entries_with_types
                                .get()
                                .iter()
                                .enumerate()
                                .map(|(tab_index, e)| {
                                    let is_active = tab_index == active_index.get();
                                    let label = e.detail.class.name.clone();
                                    view! {
                                        <a
                                            role="tab"
                                            class=format!(
                                                "tab {} {}",
                                                class_band_class(e.index),
                                                if is_active { "tab-active class-tab-active" } else { "" },
                                            )
                                            on:click=move |_| features_class_tab.set(tab_index)
                                        >
                                            {label}
                                        </a>
                                    }
                                })
                                .collect_view()}
                        </div>
                    }
                })
        }}
        {move || {
            let Some(entry) = active_entry.get() else { return ().into_any() };
            let mut types = entry.active_types.clone();
            types
                .sort_by_key(|t| {
                    OPTIONAL_FEATURE_DISPLAY_ORDER
                        .iter()
                        .position(|other| other == t)
                        .unwrap_or(usize::MAX)
                });
            // A single checklist alone in the list column leaves the fixed-width
            // panel looking stranded in a sea of empty space, so it only grows
            // to fill the row (list column shrinks to its content's natural
            // width instead) once there's just one list to show it next to.
            let (grid_class, list_wrapper_class, panel_wrapper_class) = if types.len() >= 2 {
                (
                    "grid grid-cols-1 md:grid-cols-2 gap-4",
                    "flex-1 w-full",
                    "w-full lg:w-96 lg:sticky lg:top-4",
                )
            } else {
                (
                    "grid grid-cols-1 gap-4 max-w-xl",
                    "w-full lg:w-auto",
                    "flex-1 w-full lg:sticky lg:top-4",
                )
            };
            let class_id = entry.entry.class_id.clone();
            view! {
                <div class="flex flex-col lg:flex-row gap-4 items-start">
                    <div class=list_wrapper_class>
                        <div class=grid_class>
                            {types
                                .into_iter()
                                .map(|feature_type| {
                                    optional_feature_checklist_section(
                                        class_id.clone(),
                                        feature_type,
                                        optional_feature_pool_all,
                                        feature_slots,
                                        choices,
                                        focused_feature,
                                    )
                                })
                                .collect_view()}
                        </div>
                    </div>
                    <div class=panel_wrapper_class>
                        {move || match focused_feature.get() {
                            Some(feature) => {
                                view! { <OptionalFeatureDetailCard feature=feature /> }.into_any()
                            }
                            None => {
                                detail_panel_placeholder(
                                        "Hover or select an option to see its details.",
                                    )
                                    .into_any()
                            }
                        }}
                    </div>
                </div>
            }
                .into_any()
        }}
    }
}

fn optional_feature_pool_for_class(
    optional_feature_pool_all: OptionalFeaturePoolResource,
    class_id: &str,
) -> Vec<OptionalFeature> {
    optional_feature_pool_all
        .get()
        .and_then(|result| result.ok())
        .and_then(|(_, list)| list.into_iter().find(|(id, _)| id == class_id).map(|(_, features)| features))
        .unwrap_or_default()
}

fn optional_feature_checklist_section(
    class_id: String,
    feature_type: FeatureType,
    optional_feature_pool_all: OptionalFeaturePoolResource,
    feature_slots: Memo<Vec<FeatureTypeSlot>>,
    choices: RwSignal<Vec<String>>,
    focused_feature: RwSignal<Option<OptionalFeature>>,
) -> impl IntoView {
    let slot_class_id = class_id.clone();
    let find_slot = move || {
        feature_slots
            .get()
            .into_iter()
            .find(|slot| slot.class_id == slot_class_id && slot.feature_type == feature_type)
    };
    let chosen_count_for_type = {
        let find_slot = find_slot.clone();
        move || {
            let chosen = choices.get();
            find_slot().map(|slot| chosen.iter().filter(|id| slot.eligible_ids.contains(*id)).count()).unwrap_or(0)
        }
    };

    view! {
        <div class="card bg-base-200 p-4 flex flex-col gap-2">
            <h3 class="font-semibold flex items-center gap-2">
                <span>{feature_type.label()}</span>
                <span class="badge badge-outline badge-sm font-normal">
                    {move || {
                        format!(
                            "{}/{}",
                            chosen_count_for_type(),
                            find_slot().map(|s| s.required).unwrap_or(0),
                        )
                    }}
                </span>
            </h3>
            {optional_feature_over_quota_banner(
                class_id.clone(),
                feature_type,
                feature_slots,
                choices,
            )}
            <div class="flex flex-col gap-1 max-h-96 overflow-y-auto pr-1">
                {move || {
                    optional_feature_pool_for_class(optional_feature_pool_all, &class_id)
                        .into_iter()
                        .filter(|f| f.feature_types.contains(&feature_type))
                        .map(|feature| {
                            optional_feature_choice_button(
                                class_id.clone(),
                                feature_type,
                                feature,
                                feature_slots,
                                choices,
                                focused_feature,
                            )
                        })
                        .collect_view()
                }}
            </div>
        </div>
    }
}

// Same reasoning as `over_quota_banner`: shown when a level/subclass change
// (or un-picking a Pact Boon a chosen invocation depended on) drops the
// eligible count below what's already selected.
fn optional_feature_over_quota_banner(
    class_id: String,
    feature_type: FeatureType,
    feature_slots: Memo<Vec<FeatureTypeSlot>>,
    choices: RwSignal<Vec<String>>,
) -> impl IntoView {
    move || {
        let slot = feature_slots
            .get()
            .into_iter()
            .find(|slot| slot.class_id == class_id && slot.feature_type == feature_type)?;
        let chosen_count = choices.get().iter().filter(|id| slot.eligible_ids.contains(*id)).count();
        let over = chosen_count.saturating_sub(slot.required);
        (over > 0).then(|| {
            view! {
                <div role="alert" class="alert alert-warning alert-soft text-sm py-2">
                    <span>
                        {format!(
                            "You have {over} too many selected here \u{2014} deselect {over} before continuing.",
                        )}
                    </span>
                </div>
            }
        })
    }
}

// Mirrors OptionalFeatureDetailCard's amount formatting
// (pages/optional_features.rs), minus its "Costs: " prefix — this renders as
// an already-labeled subtitle line under the feature name.
fn optional_feature_cost_label(cost: &ResourceCost) -> String {
    let amount = match (cost.amount, cost.amount_min, cost.amount_max) {
        (Some(amount), _, _) => format!("{amount} "),
        (None, Some(min), Some(max)) => format!("{min}-{max} "),
        (None, Some(min), None) => format!("{min}+ "),
        _ => String::new(),
    };
    format!("{amount}{}", cost.name)
}

// Renders every pool member of `feature_type` up front (not just the
// currently-eligible ones) — an option whose prerequisite isn't met yet
// (e.g. an invocation needing a pact not yet chosen) shows disabled rather
// than disappearing, so the player can see what's coming rather than having
// rows pop in and out as they pick a Pact Boon.
fn optional_feature_choice_button(
    class_id: String,
    feature_type: FeatureType,
    feature: OptionalFeature,
    feature_slots: Memo<Vec<FeatureTypeSlot>>,
    choices: RwSignal<Vec<String>>,
    focused_feature: RwSignal<Option<OptionalFeature>>,
) -> impl IntoView {
    let hover_feature = feature.clone();
    let click_feature = feature.clone();
    let prerequisite = prerequisite_label(&feature.prerequisites);
    let label = match &prerequisite {
        Some(prerequisite) => format!("{} ({prerequisite})", feature.name),
        None => feature.name.clone(),
    };
    let subtitle = feature.consumes.as_ref().map(optional_feature_cost_label);

    let id_for_aria = feature.id.clone();
    let id_for_class = feature.id.clone();
    let id_for_eligible_gate = feature.id.clone();
    let id_for_quota_exempt = feature.id.clone();
    let id_for_dim = feature.id.clone();
    let id_for_toggle = feature.id.clone();

    let find_slot = {
        let class_id = class_id.clone();
        move || {
            feature_slots
                .get()
                .into_iter()
                .find(|slot| slot.class_id == class_id && slot.feature_type == feature_type)
        }
    };

    let is_disabled = {
        let find_slot = find_slot.clone();
        move || {
            let Some(slot) = find_slot() else { return true };
            if !slot.eligible_ids.contains(&id_for_eligible_gate) {
                return true;
            }
            let current = choices.get();
            let count_for_type = current.iter().filter(|id| slot.eligible_ids.contains(*id)).count();
            !current.contains(&id_for_quota_exempt) && count_for_type >= slot.required
        }
    };

    view! {
        <button
            type="button"
            aria-pressed=move || choices.get().contains(&id_for_aria).to_string()
            disabled=is_disabled
            class=move || {
                if choices.get().contains(&id_for_class) {
                    "btn btn-sm btn-primary justify-start text-left h-auto py-1.5 flex-col items-start w-full"
                } else {
                    "btn btn-sm btn-outline justify-start text-left h-auto py-1.5 flex-col items-start w-full"
                }
            }
            class:opacity-50=move || {
                !find_slot().is_some_and(|slot| slot.eligible_ids.contains(&id_for_dim))
            }
            on:mouseenter=move |_| focused_feature.set(Some(hover_feature.clone()))
            on:click=move |_| {
                choices
                    .update(|list| {
                        if let Some(pos) = list.iter().position(|picked| *picked == id_for_toggle) {
                            list.remove(pos);
                        } else {
                            list.push(id_for_toggle.clone());
                        }
                    });
                focused_feature.set(Some(click_feature.clone()));
            }
        >
            <span>{label}</span>
            {subtitle
                .map(|subtitle| {
                    view! { <span class="text-xs font-normal opacity-70">{subtitle}</span> }
                })}
        </button>
    }
}

// Shown above a checklist when its choices exceed its current required count
// (e.g. a prepared caster's ability score dropped after spells were already
// picked) — the effect that clears out-of-pool picks deliberately does NOT
// also truncate down to count, so this is the only signal the player gets;
// Next stays disabled until they deselect down to `required` themselves.
fn over_quota_banner(choices: RwSignal<Vec<String>>, required: Memo<usize>) -> impl IntoView {
    move || {
        let over = choices.get().len().saturating_sub(required.get());
        (over > 0)
            .then(|| {
                view! {
                    <div role="alert" class="alert alert-warning alert-soft text-sm py-2">
                        <span>
                            {format!(
                                "You have {over} too many selected here \u{2014} deselect {over} before continuing.",
                            )}
                        </span>
                    </div>
                }
            })
    }
}

fn spell_choice_chip(
    spell: Spell,
    choices: RwSignal<Vec<String>>,
    required: Memo<usize>,
    focused_spell: RwSignal<Option<Spell>>,
) -> impl IntoView {
    let name = spell.name.clone();
    let subtitle = spell.school.label().to_string();
    let id_for_aria = spell.id.clone();
    let id_for_class = spell.id.clone();
    let id_for_disabled = spell.id.clone();
    let id_for_toggle = spell.id.clone();
    let hover_spell = spell.clone();
    let click_spell = spell;

    view! {
        <button
            type="button"
            aria-pressed=move || choices.get().contains(&id_for_aria).to_string()
            disabled=move || {
                let current = choices.get();
                !current.contains(&id_for_disabled) && current.len() >= required.get()
            }
            class=move || {
                if choices.get().contains(&id_for_class) {
                    "btn btn-sm btn-primary justify-start text-left h-auto py-1.5 flex-col items-start"
                } else {
                    "btn btn-sm btn-outline justify-start text-left h-auto py-1.5 flex-col items-start"
                }
            }
            on:mouseenter=move |_| focused_spell.set(Some(hover_spell.clone()))
            on:click=move |_| {
                focused_spell.set(Some(click_spell.clone()));
                choices
                    .update(|list| {
                        if let Some(pos) = list
                            .iter()
                            .position(|picked| *picked == id_for_toggle)
                        {
                            list.remove(pos);
                        } else {
                            list.push(id_for_toggle.clone());
                        }
                    });
            }
        >
            <span>{name}</span>
            <span class="text-xs font-normal opacity-70">{subtitle}</span>
        </button>
    }
}

fn spell_chip_section(
    title: &'static str,
    spells: Vec<Spell>,
    choices: RwSignal<Vec<String>>,
    required: Memo<usize>,
    focused_spell: RwSignal<Option<Spell>>,
) -> impl IntoView {
    view! {
        <div class="card bg-base-200 p-4 flex flex-col gap-2">
            <h3 class="font-semibold flex items-center gap-2">
                <span>{title}</span>
                <span class="badge badge-outline badge-sm font-normal">
                    {move || format!("{}/{}", choices.get().len(), required.get())}
                </span>
            </h3>
            {over_quota_banner(choices, required)}
            <div class="flex flex-wrap gap-2">
                {spells
                    .into_iter()
                    .map(|spell| spell_choice_chip(spell, choices, required, focused_spell))
                    .collect_view()}
            </div>
        </div>
    }
}

// Leveled spells (unlike cantrips) can span many spell levels at once, so
// they're grouped into per-level subsections within one card rather than one
// long flat chip row. The required-count cap and its disabled/over-quota
// logic still apply across the whole section, not per level, matching the
// real rule (one combined "spells known/prepared" count regardless of which
// levels those spells come from).
fn leveled_spell_chip_section(
    spells: Vec<Spell>,
    choices: RwSignal<Vec<String>>,
    required: Memo<usize>,
    focused_spell: RwSignal<Option<Spell>>,
) -> impl IntoView {
    let mut by_level: std::collections::BTreeMap<u8, Vec<Spell>> = std::collections::BTreeMap::new();
    for spell in spells {
        by_level.entry(spell.level).or_default().push(spell);
    }

    view! {
        <div class="card bg-base-200 p-4 flex flex-col gap-2">
            <h3 class="font-semibold flex items-center gap-2">
                <span>"Spells"</span>
                <span class="badge badge-outline badge-sm font-normal">
                    {move || format!("{}/{}", choices.get().len(), required.get())}
                </span>
            </h3>
            {over_quota_banner(choices, required)}
            <div class="flex flex-col gap-3">
                {by_level
                    .into_iter()
                    .map(|(level, spells)| {
                        view! {
                            <div class="flex flex-col gap-1">
                                <h4 class="text-sm font-semibold opacity-80">
                                    {format!("Level {level}")}
                                </h4>
                                <div class="flex flex-wrap gap-2">
                                    {spells
                                        .into_iter()
                                        .map(|spell| {
                                            spell_choice_chip(spell, choices, required, focused_spell)
                                        })
                                        .collect_view()}
                                </div>
                            </div>
                        }
                    })
                    .collect_view()}
            </div>
        </div>
    }
}

/// Compact summary of the choices made in prior steps, for the Review step —
/// see `review_summary` in `CharacterBuilderPage`. Deliberately excludes
/// fixed class/background grants (implied by the class/species/background
/// line already shown) and any full feature/spell text — this is a list of
/// names, not a character sheet.
struct ReviewSummary {
    subtitle: String,
    skill_choices: Vec<String>,
    expertise_choices: Vec<String>,
    asi_lines: Vec<String>,
    cantrips: Vec<String>,
    spells: Vec<String>,
    granted_spells: Vec<String>,
    optional_features: Vec<String>,
}

fn review_step(
    step: RwSignal<usize>,
    step_complete: impl Fn(usize) -> bool + Copy + Send + Sync + 'static,
    step_lock_reason: impl Fn(usize) -> Option<&'static str> + Copy + Send + Sync + 'static,
    build_character: impl Fn() -> Character + Copy + Send + Sync + 'static,
    resolved_entries: Memo<Vec<ResolvedClassEntry>>,
    species_detail: Resource<Result<Option<Species>, ServerFnError>>,
    review_summary: impl Fn() -> ReviewSummary + Copy + Send + Sync + 'static,
    hp_method: RwSignal<HpMethod>,
    hp_rolls: RwSignal<Vec<u8>>,
    hp_manual: RwSignal<Option<i32>>,
    saving: Memo<bool>,
    on_save: impl Fn(Character) + Copy + Send + Sync + 'static,
) -> impl IntoView {
    view! {
        <h2 class="card-title">"Review & Save"</h2>
        {move || {
            let character = build_character();
            let entries = resolved_entries.get();
            let species = species_detail.get().and_then(|result| result.ok()).flatten();
            let finals = final_abilities(&character, species.as_ref());
            let con_mod = ability_modifier(finals[2].1);
            let dex_mod = ability_modifier(finals[1].1);
            // One hit die per level, in the same order save_character
            // re-derives server-side: primary class first (its own level 1
            // is never rolled), then every later class's own levels.
            let hit_die = entries.first().map(|e| e.detail.class.hit_die).unwrap_or(8);
            let hit_dice: HashMap<String, u8> =
                entries.iter().map(|e| (e.entry.class_id.clone(), e.detail.class.hit_die)).collect();
            let hp = (!hit_dice.is_empty()).then(|| resolved_hp_max_multiclass(&character, &hit_dice, con_mod));
            let slot_hit_dice: Vec<u8> = entries
                .iter()
                .enumerate()
                .flat_map(|(index, e)| {
                    let levels = if index == 0 { e.entry.level.saturating_sub(1) } else { e.entry.level };
                    std::iter::repeat(e.detail.class.hit_die).take(levels as usize)
                })
                .collect();
            let class_lookup: HashMap<String, Class> =
                entries.iter().map(|e| (e.entry.class_id.clone(), e.detail.class.clone())).collect();
            let prereq_violations = multiclass_prereq_violations(&character, &class_lookup, species.as_ref());
            let validation = character.validate().and_then(|()| {
                if prereq_violations.is_empty() { Ok(()) } else { Err(prereq_violations.join("; ")) }
            });
            let summary = review_summary();
            // Every applicable-but-unfinished step, in step order — locked
            // steps (nothing to pick yet) don't count against completeness.
            let incomplete_steps: Vec<(usize, &'static str)> = (0..STEPS.len() - 1)
                .filter(|&i| step_lock_reason(i).is_none() && !step_complete(i))
                .map(|i| (i, STEPS[i]))
                .collect();

            view! {
                <div class="flex flex-col gap-1">
                    <h3 class="font-display text-lg font-semibold">{character.name.clone()}</h3>
                    <p class="text-sm opacity-70">{summary.subtitle.clone()}</p>
                </div>
                <div class="stat-seal grid grid-cols-3 sm:grid-cols-6 gap-2 max-w-xl p-3 border border-base-300 rounded-box bg-base-200/50">
                    {finals
                        .map(|(code, score)| {
                            view! {
                                <div class="text-center">
                                    <div class="font-mono text-[0.65rem] tracking-[0.2em] text-primary/80 uppercase">
                                        {code}
                                    </div>
                                    <div class="font-mono text-2xl font-medium leading-tight">
                                        {score.to_string()}
                                    </div>
                                    <div class="font-mono text-xs opacity-60">
                                        {format!("{:+}", ability_modifier(score))}
                                    </div>
                                </div>
                            }
                        })}
                </div>
                <div class="flex flex-wrap gap-4 text-sm font-mono">
                    <span>
                        <span class="font-sans font-semibold">"HP: "</span>
                        {hp.map(|hp| hp.to_string()).unwrap_or_else(|| "—".to_string())}
                    </span>
                    <span>
                        <span class="font-sans font-semibold">"AC: "</span>
                        {format!("{}", 10 + dex_mod as i32)}
                    </span>
                    <span>
                        <span class="font-sans font-semibold">"Proficiency: "</span>
                        {format!("+{}", proficiency_bonus(total_level(&character.classes)))}
                    </span>
                </div>
                <div class="card bg-base-200 p-4 flex flex-col gap-2 max-w-xl">
                    <h4 class="font-semibold text-sm">"Hit Points"</h4>
                    <div role="tablist" class="tabs tabs-box w-fit">
                        {[
                            (HpMethod::Average, "Average"),
                            (HpMethod::Rolled, "Rolled"),
                            (HpMethod::Manual, "Manual"),
                        ]
                            .map(|(method, label)| {
                                view! {
                                    <a
                                        role="tab"
                                        class=move || {
                                            if hp_method.get() == method { "tab tab-active" } else { "tab" }
                                        }
                                        on:click=move |_| hp_method.set(method)
                                    >
                                        {label}
                                    </a>
                                }
                            })}
                    </div>
                    {match hp_method.get() {
                        HpMethod::Average => {
                            view! {
                                <p class="text-sm opacity-70">
                                    "Uses each level's own class's average hit die (rounded up), plus your Constitution modifier."
                                </p>
                            }
                                .into_any()
                        }
                        HpMethod::Rolled => {
                            let level1 = hit_die as i32 + con_mod as i32;
                            view! {
                                <div class="flex flex-col gap-2">
                                    <p class="text-xs opacity-70">
                                        {format!("Level 1: {level1} (max d{hit_die} + Constitution modifier)")}
                                    </p>
                                    <div class="flex flex-wrap gap-3">
                                        {(2..=total_level(&character.classes))
                                            .zip(slot_hit_dice.iter().copied())
                                            .enumerate()
                                            .map(|(idx, (lvl, slot_die))| {
                                                view! {
                                                    <label class="flex flex-col items-start gap-1 text-xs">
                                                        {format!("Level {lvl} · d{slot_die}")}
                                                        <input
                                                            type="number"
                                                            class="input input-bordered input-sm w-16"
                                                            min="1"
                                                            max=slot_die.to_string()
                                                            prop:value=move || {
                                                                hp_rolls.get().get(idx).copied().unwrap_or(0).to_string()
                                                            }
                                                            on:input=move |ev| {
                                                                if let Ok(value) = event_target_value(&ev)
                                                                    .parse::<u8>()
                                                                {
                                                                    hp_rolls
                                                                        .update(|rolls| {
                                                                            if let Some(slot) = rolls.get_mut(idx) {
                                                                                *slot = value.clamp(1, slot_die);
                                                                            }
                                                                        });
                                                                }
                                                            }
                                                        />
                                                    </label>
                                                }
                                            })
                                            .collect_view()}
                                    </div>
                                </div>
                            }
                                .into_any()
                        }
                        HpMethod::Manual => {
                            view! {
                                <input
                                    type="number"
                                    class="input input-bordered input-sm w-24"
                                    min="1"
                                    prop:value=move || {
                                        hp_manual.get().map(|hp| hp.to_string()).unwrap_or_default()
                                    }
                                    on:input=move |ev| {
                                        let raw = event_target_value(&ev);
                                        hp_manual.set(raw.parse::<i32>().ok().filter(|hp| *hp > 0));
                                    }
                                />
                            }
                                .into_any()
                        }
                    }}
                </div>
                <div class="flex flex-col gap-3 max-w-xl">
                    {(!summary.skill_choices.is_empty() || !summary.expertise_choices.is_empty())
                        .then(|| {
                            view! {
                                <div class="card bg-base-200 p-4 flex flex-col gap-1">
                                    <h4 class="font-semibold text-sm">"Skills"</h4>
                                    {(!summary.skill_choices.is_empty())
                                        .then(|| {
                                            view! {
                                                <p class="text-sm">{summary.skill_choices.join(", ")}</p>
                                            }
                                        })}
                                    {(!summary.expertise_choices.is_empty())
                                        .then(|| {
                                            view! {
                                                <p class="text-sm opacity-70">
                                                    {format!(
                                                        "Expertise: {}",
                                                        summary.expertise_choices.join(", "),
                                                    )}
                                                </p>
                                            }
                                        })}
                                </div>
                            }
                        })}
                    {(!summary.cantrips.is_empty() || !summary.spells.is_empty()
                        || !summary.granted_spells.is_empty())
                        .then(|| {
                            view! {
                                <div class="card bg-base-200 p-4 flex flex-col gap-1">
                                    <h4 class="font-semibold text-sm">"Spells"</h4>
                                    {(!summary.cantrips.is_empty())
                                        .then(|| {
                                            view! {
                                                <p class="text-sm">
                                                    {format!("Cantrips: {}", summary.cantrips.join(", "))}
                                                </p>
                                            }
                                        })}
                                    {(!summary.spells.is_empty())
                                        .then(|| {
                                            view! {
                                                <p class="text-sm">
                                                    {format!("Known/Prepared: {}", summary.spells.join(", "))}
                                                </p>
                                            }
                                        })}
                                    {(!summary.granted_spells.is_empty())
                                        .then(|| {
                                            view! {
                                                <p class="text-sm opacity-70">
                                                    {format!(
                                                        "Granted (always prepared): {}",
                                                        summary.granted_spells.join(", "),
                                                    )}
                                                </p>
                                            }
                                        })}
                                </div>
                            }
                        })}
                    {(!summary.optional_features.is_empty())
                        .then(|| {
                            view! {
                                <div class="card bg-base-200 p-4 flex flex-col gap-1">
                                    <h4 class="font-semibold text-sm">"Optional Features"</h4>
                                    <p class="text-sm">{summary.optional_features.join(", ")}</p>
                                </div>
                            }
                        })}
                    {(!summary.asi_lines.is_empty())
                        .then(|| {
                            view! {
                                <div class="card bg-base-200 p-4 flex flex-col gap-1">
                                    <h4 class="font-semibold text-sm">
                                        "Ability Score Improvements"
                                    </h4>
                                    {summary
                                        .asi_lines
                                        .iter()
                                        .map(|line| view! { <p class="text-sm">{line.clone()}</p> })
                                        .collect_view()}
                                </div>
                            }
                        })}
                </div>
                {(!incomplete_steps.is_empty())
                    .then(|| {
                        view! {
                            <div class="alert alert-warning flex flex-col items-start gap-2 max-w-xl">
                                <span class="font-semibold text-sm">"Still needs:"</span>
                                <div class="flex flex-wrap gap-2">
                                    {incomplete_steps
                                        .iter()
                                        .map(|&(index, title)| {
                                            view! {
                                                <button
                                                    type="button"
                                                    class="btn btn-xs btn-outline"
                                                    on:click=move |_| step.set(index)
                                                >
                                                    {title}
                                                </button>
                                            }
                                        })
                                        .collect_view()}
                                </div>
                            </div>
                        }
                    })}
                {validation
                    .as_ref()
                    .err()
                    .map(|err| view! { <p class="text-warning text-sm">{err.clone()}</p> })}
                <button
                    type="button"
                    class="btn btn-primary w-fit"
                    disabled=validation.is_err() || saving.get() || !incomplete_steps.is_empty()
                    on:click=move |_| on_save(build_character())
                >
                    {if saving.get() { "Saving..." } else { "Save Character" }}
                </button>
            }
        }}
    }
}

#[cfg(test)]
#[path = "character_builder_tests.rs"]
mod tests;
