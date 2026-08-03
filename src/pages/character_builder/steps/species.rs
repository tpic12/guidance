use crate::components::species_detail_card::SpeciesDetailCard;
use crate::models::species::Species;
use leptos::prelude::*;

use super::super::utils::{group_species_by_name, species_subtitle};
use super::detail_panel_placeholder;

pub fn species_step(
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
                            view! { <li class="p-2 text-sm opacity-60">"Loading..."</li> }
                                .into_any()
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
                            view! { <li class="p-2 text-sm opacity-60">"No matches."</li> }
                                .into_any()
                        }
                        Some(Ok(rows)) => {
                            group_species_by_name(rows)
                                .into_iter()
                                .map(|(name, variants)| {
                                    if variants.len() <= 1 {
                                        let species = variants.into_iter().next().unwrap();
                                        let label = species.name.clone();
                                        species_row(species, label, species_id, focused_species)
                                            .into_any()
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
                        "flex flex-col items-start gap-0 bg-primary/15 border border-primary/50"
                    } else {
                        "flex flex-col items-start gap-0 border border-transparent"
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
                                                let label = format!(
                                                    "{} ({})",
                                                    species.name,
                                                    species.source,
                                                );
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
