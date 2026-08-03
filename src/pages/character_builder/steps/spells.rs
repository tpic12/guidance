use crate::components::spell_detail_card::SpellDetailCard;
use crate::models::spell::Spell;
use leptos::prelude::*;

use super::super::{class_band_class, EntryKey, ResolvedClassEntry, SpellChoiceSlot, SpellRequirement};
use super::detail_panel_placeholder;

pub fn spells_step(
    resolved_entries: Memo<Vec<ResolvedClassEntry>>,
    spell_options_all: Resource<Result<(Vec<EntryKey>, Vec<(String, Vec<Spell>)>), ServerFnError>>,
    granted_spells_all: Resource<Result<Vec<(String, Vec<Spell>)>, ServerFnError>>,
    spell_requirements: Memo<Vec<SpellRequirement>>,
    cantrip_choices: RwSignal<Vec<String>>,
    spell_choices: RwSignal<Vec<String>>,
    spells_class_tab: RwSignal<usize>,
    spell_choice_slots: Memo<Vec<SpellChoiceSlot>>,
    spell_grant_choices: RwSignal<Vec<Option<String>>>,
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
                                let spells = list
                                    .into_iter()
                                    .find(|(id, _)| *id == class_id)
                                    .map(|(_, s)| s)
                                    .unwrap_or_default();
                                if spells.is_empty() {
                                    return ().into_any();
                                }
                                let names = spells
                                    .iter()
                                    .map(|spell| spell.name.clone())
                                    .collect::<Vec<_>>()
                                    .join(", ");
                                view! {
                                    <div
                                        role="alert"
                                        class="alert alert-info alert-soft text-sm py-2"
                                    >
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
                <For
                    each=move || {
                        let Some(class_id) = active_class_id.get() else { return Vec::new() };
                        spell_choice_slots
                            .get()
                            .into_iter()
                            .enumerate()
                            .filter(|(_, slot)| slot.class_id == class_id)
                            .collect::<Vec<_>>()
                    }
                    key=|(slot, _)| *slot
                    children=move |(slot, choice_slot)| {
                        spell_choice_slot(
                            slot,
                            choice_slot,
                            spell_grant_choices,
                            cantrip_choices,
                            spell_choices,
                            focused_spell,
                        )
                    }
                />
                <Suspense fallback=move || {
                    view! { <p>"Loading spells..."</p> }
                }>
                    {move || Suspend::new(async move {
                        let Some(class_id) = active_class_id.get() else { return ().into_any() };
                        match spell_options_all.await {
                            Ok((_, list)) => {
                                let spells = list
                                    .into_iter()
                                    .find(|(id, _)| *id == class_id)
                                    .map(|(_, s)| s)
                                    .unwrap_or_default();
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
                                    <p class="text-error">
                                        {format!("Failed to load spells: {err}")}
                                    </p>
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
                        detail_panel_placeholder("Hover or select a spell to see its details.")
                            .into_any()
                    }
                }}
            </div>
        </div>
    }
}

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
                        if let Some(pos) = list.iter().position(|picked| *picked == id_for_toggle) {
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

/// One labelled dropdown for a subclass spell grant slot — same shape as `skill_slot`.
fn spell_choice_slot(
    slot: usize,
    choice_slot: SpellChoiceSlot,
    spell_grant_choices: RwSignal<Vec<Option<String>>>,
    cantrip_choices: RwSignal<Vec<String>>,
    spell_choices: RwSignal<Vec<String>>,
    focused_spell: RwSignal<Option<Spell>>,
) -> impl IntoView {
    let choice = move || spell_grant_choices.get().get(slot).cloned().flatten();
    let set_choice = move |value: Option<String>| {
        spell_grant_choices.update(|choices| {
            if let Some(entry) = choices.get_mut(slot) {
                *entry = value;
            }
        });
    };
    let level_label = choice_slot
        .options
        .first()
        .map(|s| if s.level == 0 { "cantrip".to_string() } else { format!("level {} spell", s.level) })
        .unwrap_or_else(|| "spell".to_string());
    let heading = format!("{}: choose a {level_label}", choice_slot.source);
    let options = choice_slot.options;
    let focus_options = options.clone();
    let is_disabled = move |spell_id: &str| {
        spell_grant_choices
            .get()
            .iter()
            .enumerate()
            .any(|(i, picked)| i != slot && picked.as_deref() == Some(spell_id))
            || cantrip_choices.get().iter().any(|id| id == spell_id)
            || spell_choices.get().iter().any(|id| id == spell_id)
    };

    view! {
        <div class="flex flex-col gap-1 p-3 border border-base-300 rounded-box">
            <span class="font-semibold text-sm">{heading}</span>
            <select
                class="select select-bordered select-sm w-64"
                on:change=move |ev| {
                    let value = event_target_value(&ev);
                    set_choice(if value.is_empty() { None } else { Some(value) });
                }
                on:focus=move |_| {
                    if let Some(picked) = choice() {
                        focused_spell.set(focus_options.iter().find(|s| s.id == picked).cloned());
                    }
                }
            >
                <option value="" selected=move || choice().is_none()>
                    "— choose —"
                </option>
                {options
                    .into_iter()
                    .map(|spell| {
                        let value = spell.id.clone();
                        let label = spell.name.clone();
                        let selected_id = spell.id.clone();
                        let disabled_id = spell.id.clone();
                        let hover_spell = spell.clone();
                        view! {
                            <option
                                value=value
                                selected=move || choice().as_deref() == Some(selected_id.as_str())
                                disabled=move || is_disabled(&disabled_id)
                                on:mouseenter=move |_| focused_spell.set(Some(hover_spell.clone()))
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
