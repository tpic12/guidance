mod actions_tab;
mod badges;
mod features_tab;
mod placeholder_tab;
mod spells_tab;

use crate::components::entry_view::entry_view;
use crate::models::character::{
    ability_modifier, active_subclass, asi_levels, effective_spellcasting, final_abilities, language_slots,
    multiclass_class_skill_grants, multiclass_pact_slots, multiclass_spell_slots, proficiency_bonus,
    resolved_hp_max_multiclass, skill_slots, spell_attack_bonus, spell_save_dc, tool_slots, total_level, AsiChoice,
    CharacterSheet, ClassLevel, SpellcastingProfile, ABILITY_CODES,
};
use crate::models::class::{ability_label, Class, ClassDetail, ClassFeature, SubclassDetail};
use crate::models::language::LanguageGrant;
use crate::models::proficiency::{merge_resolved_names, ToolGrant};
use crate::models::skill::{skill_label, SkillGrant, SKILLS};
use crate::models::spell::Spell;
use actions_tab::ActionsTab;
use features_tab::FeaturesTab;
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use placeholder_tab::PlaceholderTab;
use spells_tab::SpellsTab;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SheetTab {
    Actions,
    Spells,
    Features,
    Inventory,
    Notes,
}

/// One tab-bar entry: renders active/inactive and flips `active` on click.
/// Shared by the top-level sheet tabs and every tab's own sub-tab bar.
fn tab_link<T>(label: &'static str, tab: T, active: RwSignal<T>) -> impl IntoView
where
    T: Copy + PartialEq + Send + Sync + 'static,
{
    view! {
        <a
            role="tab"
            class="tab"
            class:tab-active=move || active.get() == tab
            on:click=move |_| active.set(tab)
        >
            {label}
        </a>
    }
}

#[server]
pub async fn get_character_sheet(id: String) -> Result<Option<CharacterSheet>, ServerFnError> {
    use sqlx::SqlitePool;
    use tower_sessions::Session;

    let pool = expect_context::<SqlitePool>();
    let session: Session = leptos_axum::extract().await?;
    let user = crate::auth::require_login(&pool, &session).await?;
    crate::db::get_character_sheet(&pool, &id, &user.id)
        .await
        .map_err(|err| ServerFnError::new(err.to_string()))
}

#[component]
pub fn CharacterSheetPage() -> impl IntoView {
    let params = use_params_map();
    let sheet = Resource::new(
        move || params.read().get("id").unwrap_or_default(),
        |id| async move { get_character_sheet(id).await },
    );

    view! {
        <div class="flex flex-col gap-4">
            <a href="/characters" class="btn btn-ghost btn-sm w-fit gap-2">
                "← Back to Characters"
            </a>
            <Suspense fallback=move || {
                view! { <p>"Loading character..."</p> }
            }>
                {move || Suspend::new(async move {
                    match sheet.await {
                        Ok(Some(sheet)) => view! { <SheetView sheet=sheet /> }.into_any(),
                        Ok(None) => {
                            view! { <p class="opacity-70">"Character not found."</p> }.into_any()
                        }
                        Err(err) => {
                            view! {
                                <p class="text-error">
                                    {format!("Failed to load character: {err}")}
                                </p>
                            }
                                .into_any()
                        }
                    }
                })}
            </Suspense>
        </div>
    }
}

/// One `character.classes` entry, resolved against `sheet.classes` and its own subclass detail.
struct ResolvedClass<'a> {
    entry: &'a ClassLevel,
    detail: &'a ClassDetail,
    subclass: Option<&'a SubclassDetail>,
}

#[component]
fn SheetView(sheet: CharacterSheet) -> impl IntoView {
    let character = sheet.character.clone();
    let finals = final_abilities(&character, sheet.species.as_ref());
    let score_of = |code: &str| finals.iter().find(|(c, _)| *c == code).map(|(_, s)| *s).unwrap_or(10);
    let dex_mod = ability_modifier(score_of("dex"));
    let con_mod = ability_modifier(score_of("con"));
    let total_level = total_level(&character.classes);
    let prof = proficiency_bonus(total_level);

    // A purged/archived class simply isn't in `sheet.classes` (tolerated by
    // `db::get_character_sheet`), so it's skipped here rather than failing
    // the whole sheet — matching the single-class tolerate-dangling behavior
    // this app has always had.
    let resolved_classes: Vec<ResolvedClass> = character
        .classes
        .iter()
        .filter_map(|entry| {
            let detail = sheet.classes.iter().find(|d| d.class.id == entry.class_id)?;
            let subclass = active_subclass(detail, entry.subclass_id.as_deref(), entry.level)
                .and_then(|active| detail.subclasses.iter().find(|sd| sd.subclass.id == active.id));
            Some(ResolvedClass { entry, detail, subclass })
        })
        .collect();

    let subtitle = {
        let mut parts = Vec::new();
        match resolved_classes.as_slice() {
            [] => parts.push(format!("Level {total_level} Unknown class")),
            [rc] => {
                parts.push(format!("Level {total_level} {}", rc.detail.class.name));
                if let Some(sub) = rc.subclass {
                    parts.push(sub.subclass.short_name.clone());
                }
            }
            many => {
                let segments: Vec<String> = many
                    .iter()
                    .map(|rc| {
                        let mut segment = format!("Level {} {}", rc.entry.level, rc.detail.class.name);
                        if let Some(sub) = rc.subclass {
                            segment.push_str(&format!(" ({})", sub.subclass.short_name));
                        }
                        segment
                    })
                    .collect();
                parts.push(segments.join(" / "));
            }
        }
        if let Some(species) = &sheet.species {
            parts.push(species.name.clone());
        }
        if let Some(background) = &sheet.background {
            parts.push(background.name.clone());
        }
        parts.join(" · ")
    };

    let hp = (!resolved_classes.is_empty())
        .then(|| {
            let hit_dice: HashMap<String, u8> =
                resolved_classes.iter().map(|rc| (rc.detail.class.id.clone(), rc.detail.class.hit_die)).collect();
            resolved_hp_max_multiclass(&character, &hit_dice, con_mod).to_string()
        })
        .unwrap_or_else(|| "—".to_string());
    let speed = sheet
        .species
        .as_ref()
        .and_then(|species| species.speed.clone())
        .unwrap_or_else(|| "—".to_string());
    let darkvision = sheet
        .species
        .as_ref()
        .and_then(|species| species.darkvision)
        .map(|range| format!("{range} ft."));

    // Saving throw proficiencies only ever come from a character's first
    // (primary) class in 5e — multiclassing into a second class doesn't add
    // any more.
    let saving_throws = {
        let proficient = resolved_classes.first().map(|rc| rc.detail.class.saving_throws.clone()).unwrap_or_default();
        ABILITY_CODES
            .map(|code| {
                let is_proficient = proficient.iter().any(|c| c == code);
                let modifier = ability_modifier(score_of(code))
                    + if is_proficient { prof as i8 } else { 0 };
                (code, modifier, is_proficient)
            })
    };

    let skill_rows = {
        let class_lookup: HashMap<String, Class> =
            resolved_classes.iter().map(|rc| (rc.detail.class.id.clone(), rc.detail.class.clone())).collect();
        let class_skills = multiclass_class_skill_grants(&character.classes, &class_lookup);
        let background_skills: Vec<(String, SkillGrant)> = sheet
            .background
            .as_ref()
            .map(|background| background.skills.iter().cloned().map(|grant| (background.name.clone(), grant)).collect())
            .unwrap_or_default();
        let slots = skill_slots(&class_skills, &background_skills);
        let mut proficient: HashSet<String> = slots.fixed.into_iter().collect();
        proficient.extend(character.skill_choices.iter().cloned());
        proficient.extend(character.custom_skill_proficiencies.iter().cloned());
        let expertise: HashSet<&String> = character.expertise_choices.iter().collect();
        SKILLS.map(|(name, ability)| {
            let is_proficient = proficient.contains(name);
            let is_expertise = expertise.contains(&name.to_string());
            let modifier = ability_modifier(score_of(ability))
                + if is_proficient { prof as i8 } else { 0 }
                + if is_expertise { prof as i8 } else { 0 };
            (name, modifier, is_proficient, is_expertise)
        })
    };

    // ASI slots are granted per class, gated on that class's own level — but
    // `asi_choices` is one flat list in `classes` order (see
    // `multiclass_asi_slots`), so labels are assigned by walking that same
    // order. To handle dangling classes gracefully, build a stable map keyed
    // by (class_id, asi_level) to avoid misaligning choices when classes are
    // removed.
    let ability_asis: Vec<String> = {
        // Build a stable map from (class_id, asi_level) -> AsiChoice
        let mut asi_map: HashMap<(String, u8), Option<AsiChoice>> = HashMap::new();
        let mut choice_idx = 0;
        for entry in &character.classes {
            // Fetch the ClassDetail for this class to get its features
            if let Some(detail) = sheet.classes.iter().find(|d| d.class.id == entry.class_id) {
                for asi_level in asi_levels(&detail.features)
                    .into_iter()
                    .filter(|&l| l <= entry.level)
                {
                    if choice_idx < character.asi_choices.len() {
                        asi_map.insert((entry.class_id.clone(), asi_level), character.asi_choices[choice_idx].clone());
                        choice_idx += 1;
                    }
                }
            }
        }
        // Display labels only for resolved classes
        let mut labels = Vec::new();
        for rc in &resolved_classes {
            for asi_level in asi_levels(&rc.detail.features).into_iter().filter(|&l| l <= rc.entry.level) {
                if let Some(Some(AsiChoice::Abilities { codes })) =
                    asi_map.get(&(rc.detail.class.id.clone(), asi_level))
                {
                    let increases =
                        codes.iter().map(|code| format!("+1 {}", ability_label(code))).collect::<Vec<_>>().join(", ");
                    labels.push(if resolved_classes.len() > 1 {
                        format!("{} level {asi_level}: {increases}", rc.detail.class.name)
                    } else {
                        format!("Level {asi_level}: {increases}")
                    });
                }
            }
        }
        labels
    };

    let class_feature_sections: Vec<AnyView> = resolved_classes
        .iter()
        .filter_map(|rc| {
            let features: Vec<ClassFeature> =
                rc.detail.features.iter().filter(|f| f.level <= rc.entry.level).cloned().collect();
            (!features.is_empty()).then(|| {
                feature_section(
                    format!("{} Features", rc.detail.class.name),
                    features
                        .iter()
                        .map(|feature| {
                            (
                                Some(format!("{} (level {})", feature.name, feature.level)),
                                feature.entries.iter().map(entry_view).collect_view().into_any(),
                            )
                        })
                        .collect(),
                )
                .into_any()
            })
        })
        .collect();
    let subclass_feature_sections: Vec<AnyView> = resolved_classes
        .iter()
        .filter_map(|rc| {
            let sub = rc.subclass?;
            let features: Vec<ClassFeature> =
                sub.features.iter().filter(|f| f.level <= rc.entry.level).cloned().collect();
            (!features.is_empty()).then(|| {
                feature_section(
                    format!("{} Features", sub.subclass.name),
                    features
                        .iter()
                        .map(|feature| {
                            (
                                Some(format!("{} (level {})", feature.name, feature.level)),
                                feature.entries.iter().map(entry_view).collect_view().into_any(),
                            )
                        })
                        .collect(),
                )
                .into_any()
            })
        })
        .collect();

    // Every spell that can be cast, tagged with whether it's a subclass/feat
    // grant (always prepared) — shared by the Actions tab (filtered by cast
    // time) and the Spells tab (grouped by level).
    let all_castables: Vec<(Spell, bool)> = sheet
        .cantrips
        .into_iter()
        .chain(sheet.spells)
        .map(|spell| (spell, false))
        .chain(sheet.granted_spells.into_iter().map(|spell| (spell, true)))
        .collect();

    let progressions: HashMap<String, Option<String>> = resolved_classes
        .iter()
        .map(|rc| {
            let profile = effective_spellcasting(&rc.detail.class, rc.subclass.map(|sd| &sd.subclass));
            (rc.entry.class_id.clone(), profile.caster_progression)
        })
        .collect();
    let slot_lines: Vec<String> = multiclass_spell_slots(&character.classes, &progressions)
        .iter()
        .enumerate()
        .filter(|(_, count)| **count > 0)
        .map(|(level, count)| format!("Level {}: {count}", level + 1))
        .collect();
    let pact_slot_lines: Vec<String> = multiclass_pact_slots(&character.classes, &progressions)
        .iter()
        .enumerate()
        .filter(|(_, count)| **count > 0)
        .map(|(level, count)| format!("Level {}: {count}", level + 1))
        .collect();
    let caster_lines: Vec<(String, SpellcastingProfile)> = resolved_classes
        .iter()
        .filter_map(|rc| {
            let profile = effective_spellcasting(&rc.detail.class, rc.subclass.map(|sd| &sd.subclass));
            profile.caster_progression.as_ref()?;
            Some((rc.detail.class.name.clone(), profile))
        })
        .collect();
    let multiple_casters = caster_lines.len() > 1;
    // The shared slot pool comes from every non-Pact caster class combined
    // (`multiclass_spell_slots` already special-cases a solo half/third
    // caster to use their own table); Pact Magic (Warlock) is tracked as its
    // own separate row, never merged in.
    let casting_lines: Vec<String> = caster_lines
        .iter()
        .filter_map(|(class_name, profile)| {
            let code = profile.spellcasting_ability.as_deref()?;
            let ability_mod = ability_modifier(score_of(code));
            let line = format!(
                "Save DC {} · Attack {:+}",
                spell_save_dc(prof, ability_mod),
                spell_attack_bonus(prof, ability_mod),
            );
            Some(if multiple_casters { format!("{class_name}: {line}") } else { line })
        })
        .collect();

    let asi_view = (!ability_asis.is_empty()).then(|| {
        view! {
            <div class="card bg-base-100 border border-base-300">
                <div class="card-body py-3 gap-1">
                    <h2 class="font-mono text-[0.7rem] tracking-[0.15em] uppercase text-primary/80 pb-1 border-b border-base-300">
                        "Ability Score Improvements"
                    </h2>
                    {ability_asis.into_iter().map(|line| view! { <p class="text-sm">{line}</p> }).collect_view()}
                </div>
            </div>
        }
    });

    let proficiencies_view = view! {
        <div class="card bg-base-100 border border-base-300">
            <div class="card-body py-3 gap-2">
                <h2 class="font-mono text-[0.7rem] tracking-[0.15em] uppercase text-primary/80 pb-1 border-b border-base-300">
                    "Proficiencies & Training"
                </h2>
                {(!resolved_classes.is_empty())
                    .then(|| {
                        let mut armor = Vec::new();
                        let mut weapons = Vec::new();
                        // Class + background grants combined, same as `tool_inputs` in the
                        // wizard — fixed grants merged with the player's actual picks
                        // (unlike Armor/Weapons, which have no player-choice slots).
                        let mut tool_grants: Vec<(String, ToolGrant)> = Vec::new();
                        for (index, rc) in resolved_classes.iter().enumerate() {
                            let profs = if index == 0 {
                                &rc.detail.class.proficiencies
                            } else {
                                &rc.detail.class.multiclass_proficiencies
                            };
                            armor.extend(profs.armor.iter().cloned());
                            weapons.extend(profs.weapons.iter().cloned());
                            tool_grants.extend(profs.tools.iter().cloned().map(|g| (rc.detail.class.name.clone(), g)));
                        }
                        if let Some(background) = &sheet.background {
                            tool_grants
                                .extend(background.tools.iter().cloned().map(|g| (background.name.clone(), g)));
                        }
                        for list in [&mut armor, &mut weapons] {
                            let mut seen = HashSet::new();
                            list.retain(|item| seen.insert(item.clone()));
                        }
                        let tool_choices: Vec<String> = character
                            .tool_choices
                            .iter()
                            .chain(character.custom_tool_proficiencies.iter())
                            .cloned()
                            .collect();
                        let tools = merge_resolved_names(
                            &tool_slots(&tool_grants, &[], &HashMap::new()).fixed,
                            &tool_choices,
                        );
                        let line = |label: &str, value: String| {
                            (!value.is_empty())
                                .then(|| {
                                    let label = label.to_string();
                                    // Only the primary class grants its full armor/weapon/tool set — later classes get `multiclass_proficiencies`, same rule as skills.
                                    view! {
                                        <p class="text-sm">
                                            <span class="font-semibold">{label} ": "</span>
                                            {value}
                                        </p>
                                    }
                                })
                        };
                        view! {
                            {line("Armor", armor.join(", "))}
                            {line("Weapons", weapons.join(", "))}
                            {line("Tools", tools.join(", "))}
                        }
                    })}
                {{
                    // Background + species grants combined, same as `language_inputs`
                    // in the wizard — fixed grants merged with the player's picks.
                    let mut language_grants: Vec<(String, LanguageGrant)> = Vec::new();
                    if let Some(background) = &sheet.background {
                        language_grants
                            .extend(background.languages.iter().cloned().map(|g| (background.name.clone(), g)));
                    }
                    if let Some(species) = &sheet.species {
                        language_grants
                            .extend(species.languages.iter().cloned().map(|g| (species.name.clone(), g)));
                    }
                    let language_choices: Vec<String> = character
                        .language_choices
                        .iter()
                        .chain(character.custom_language_proficiencies.iter())
                        .cloned()
                        .collect();
                    let languages = merge_resolved_names(
                        &language_slots(&language_grants, &[], &[]).fixed,
                        &language_choices,
                    );
                    (!languages.is_empty())
                        .then(|| {
                            view! {
                                <p class="text-sm">
                                    <span class="font-semibold">"Languages: "</span>
                                    {languages.join(", ")}
                                </p>
                            }
                        })
                }}
            </div>
        </div>
    };
    let summary_view = view! { <div class="flex flex-col gap-3">{asi_view} {proficiencies_view}</div> }.into_any();

    let species_view = sheet.species.as_ref().filter(|species| !species.entries.is_empty()).map(|species| {
        feature_section(
            format!("{} Traits", species.name),
            species.entries.iter().map(|entry| (None, entry_view(entry).into_any())).collect(),
        )
        .into_any()
    });

    let feats_view = (!sheet.feats.is_empty()).then(|| {
        feature_section(
            "Feats".to_string(),
            sheet
                .feats
                .iter()
                .map(|feat| {
                    (
                        Some(format!("{} ({})", feat.name, feat.source)),
                        feat.entries.iter().map(entry_view).collect_view().into_any(),
                    )
                })
                .collect(),
        )
        .into_any()
    });

    let optional_view = (!sheet.optional_features.is_empty()).then(|| {
        feature_section(
            "Optional Features".to_string(),
            sheet
                .optional_features
                .iter()
                .map(|feature| {
                    let types = feature.feature_types.iter().map(|t| t.label()).collect::<Vec<_>>().join(", ");
                    (
                        Some(format!("{} ({}) — {}", feature.name, feature.source, types)),
                        feature.entries.iter().map(entry_view).collect_view().into_any(),
                    )
                })
                .collect(),
        )
        .into_any()
    });

    let active_tab = RwSignal::new(SheetTab::Actions);

    view! {
        <div class="flex flex-col gap-1">
            <h1 class="font-display text-3xl font-semibold">{character.name.clone()}</h1>
            <p class="opacity-70">{subtitle}</p>
        </div>

        <div class="stat-seal grid grid-cols-3 sm:grid-cols-6 gap-2 max-w-3xl p-3 border border-base-300 rounded-box bg-base-200/50">
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

        <div class="flex flex-wrap gap-x-6 gap-y-2 text-sm font-mono">
            <span>
                <span class="font-sans font-semibold">"AC "</span>
                {format!("{}", 10 + dex_mod as i32)}
            </span>
            <span>
                <span class="font-sans font-semibold">"HP "</span>
                {hp}
            </span>
            <span>
                <span class="font-sans font-semibold">"Proficiency "</span>
                {format!("+{prof}")}
            </span>
            <span>
                <span class="font-sans font-semibold">"Initiative "</span>
                {format!("{dex_mod:+}")}
            </span>
            <span>
                <span class="font-sans font-semibold">"Speed "</span>
                {speed}
            </span>
            {darkvision
                .map(|darkvision| {
                    view! {
                        <span>
                            <span class="font-sans font-semibold">"Darkvision "</span>
                            {darkvision}
                        </span>
                    }
                })}
        </div>

        <div class="card bg-base-100 border border-base-300">
            <div class="card-body py-3 gap-1">
                <h2 class="font-mono text-[0.7rem] tracking-[0.15em] uppercase text-primary/80 pb-1 border-b border-base-300">
                    "Saving Throws"
                </h2>
                <div class="flex flex-wrap gap-x-4 gap-y-1 text-sm">
                    {saving_throws
                        .map(|(code, modifier, is_proficient)| {
                            let label = format!(
                                "{} {modifier:+}{}",
                                ability_label(code),
                                if is_proficient { " ●" } else { "" },
                            );
                            view! { <span>{label}</span> }
                        })}
                </div>
            </div>
        </div>

        <div class="card bg-base-100 border border-base-300">
            <div class="card-body py-3 gap-1">
                <h2 class="font-mono text-[0.7rem] tracking-[0.15em] uppercase text-primary/80 pb-1 border-b border-base-300">
                    "Skills"
                </h2>
                <div class="flex flex-wrap gap-x-4 gap-y-1 text-sm">
                    {skill_rows
                        .map(|(name, modifier, is_proficient, is_expertise)| {
                            let marker = if is_expertise {
                                " ★"
                            } else if is_proficient {
                                " ●"
                            } else {
                                ""
                            };
                            let label = format!("{} {modifier:+}{marker}", skill_label(name));
                            view! { <span>{label}</span> }
                        })}
                </div>
            </div>
        </div>

        <div role="tablist" class="tabs tabs-lift mt-2 flex-wrap">
            {tab_link("Actions", SheetTab::Actions, active_tab)}
            {tab_link("Spells", SheetTab::Spells, active_tab)}
            {tab_link("Features", SheetTab::Features, active_tab)}
            {tab_link("Inventory", SheetTab::Inventory, active_tab)}
            {tab_link("Notes", SheetTab::Notes, active_tab)}
        </div>

        <div class="pt-2" class:hidden=move || active_tab.get() != SheetTab::Actions>
            <ActionsTab all_castables=all_castables.clone() />
        </div>
        <div class="pt-2" class:hidden=move || active_tab.get() != SheetTab::Spells>
            <SpellsTab
                casting_lines=casting_lines
                slot_lines=slot_lines
                pact_slot_lines=pact_slot_lines
                all_castables=all_castables
            />
        </div>
        <div class="pt-2" class:hidden=move || active_tab.get() != SheetTab::Features>
            <FeaturesTab
                summary=summary_view
                species=species_view
                class_sections=class_feature_sections
                subclass_sections=subclass_feature_sections
                feats=feats_view
                optional=optional_view
            />
        </div>
        <div class="pt-2" class:hidden=move || active_tab.get() != SheetTab::Inventory>
            <PlaceholderTab
                title="Inventory isn't tracked yet"
                body="Guidance doesn't yet have a way to attach items to a character. This tab is reserved for that — check back in a future update."
            />
        </div>
        <div class="pt-2" class:hidden=move || active_tab.get() != SheetTab::Notes>
            <PlaceholderTab
                title="Notes aren't tracked yet"
                body="There's no place to jot down character notes yet — this tab is reserved for that."
            />
        </div>
    }
}

fn feature_section(title: String, blocks: Vec<(Option<String>, AnyView)>) -> impl IntoView {
    view! {
        <div class="collapse collapse-arrow bg-base-100 border border-base-300">
            <input type="checkbox" checked=true />
            <div class="collapse-title font-display font-semibold">{title}</div>
            <div class="collapse-content space-y-3 text-sm">
                {blocks
                    .into_iter()
                    .map(|(heading, body)| {
                        view! {
                            <div class="space-y-1">
                                {heading
                                    .map(|heading| {
                                        view! {
                                            <h3 class="font-mono text-xs tracking-[0.1em] uppercase text-primary/80">
                                                {heading}
                                            </h3>
                                        }
                                    })} {body}
                            </div>
                        }
                    })
                    .collect_view()}
            </div>
        </div>
    }
}
