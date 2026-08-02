use crate::components::optional_feature_detail_card::OptionalFeatureDetailCard;
use crate::models::optional_feature::{prerequisite_label, FeatureType, OptionalFeature};
use leptos::prelude::*;

use super::super::{class_band_class, EntryKey, FeatureTypeSlot, ResolvedClassEntry};
use super::super::utils::optional_feature_cost_label;
use super::detail_panel_placeholder;

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

pub type OptionalFeaturePoolResource =
    Resource<Result<(Vec<EntryKey>, Vec<(String, Vec<OptionalFeature>)>), ServerFnError>>;

// One sub-tab per class that currently grants any optional features (only shown once there's more than one).
pub fn optional_features_step(
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
            // A single checklist alone in the list column leaves the fixed-width
            // panel looking stranded in a sea of empty space, so it only grows
            // to fill the row (list column shrinks to its content's natural
            // width instead) once there's just one list to show it next to.
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

// Same reasoning as spells' `over_quota_banner`: shown when a level/subclass
// change (or un-picking a Pact Boon a chosen invocation depended on) drops
// the eligible count below what's already selected.
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
