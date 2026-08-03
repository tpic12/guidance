use crate::components::feat_detail_card::FeatDetailCard;
use crate::models::character::{AsiChoice, ABILITY_CODES};
use crate::models::class::ability_label;
use crate::models::feat::Feat;
use leptos::prelude::*;

use super::super::{class_band_class, ResolvedClassEntry};
use super::detail_panel_placeholder;

pub fn asi_step(
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
            let indexed_slots: Vec<(usize, usize, u8)> = asi_entries
                .get()
                .into_iter()
                .enumerate()
                .map(|(slot, (entry_index, _, slot_level))| (slot, entry_index, slot_level))
                .collect();
            if indexed_slots.is_empty() {
                return // Global slot position (indexes the flat `asi_choices` vec)
                // alongside which resolved entry granted it and at what level.
                view! {
                    <p class="text-sm opacity-70">
                        "No improvement slots at this level — most classes unlock their first at level 4."
                    </p>
                }
                    .into_any();
            }
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
            // Only tab classes that actually have an unlocked slot.
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
