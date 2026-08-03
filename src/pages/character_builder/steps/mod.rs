mod abilities;
mod asi;
mod background;
mod basics;
mod class;
mod languages;
mod optional_features;
mod review;
mod skills;
mod species;
mod spells;
mod tools;

pub use abilities::abilities_step;
pub use asi::asi_step;
pub use background::background_step;
pub use basics::basics_step;
pub use class::class_step;
pub use languages::languages_step;
pub use optional_features::{optional_features_step, OptionalFeaturePoolResource};
pub use review::{review_step, ReviewSummary};
pub use skills::skills_step;
pub use species::species_step;
pub use spells::spells_step;
pub use tools::tools_step;

use leptos::prelude::*;

// Shared empty state for the Background/Species/Spells/ASI/Optional
// Features steps' right-side detail panel, before anything's been
// hovered/selected yet.
fn detail_panel_placeholder(message: &'static str) -> impl IntoView {
    view! {
        <div class="card bg-base-200 border border-base-300 border-dashed">
            <div class="card-body items-center text-center py-8">
                <p class="text-sm opacity-60">{message}</p>
            </div>
        </div>
    }
}

/// Shared "add a proficiency the app didn't grant" control for the
/// Skills/Languages/Tools steps — an escape valve for prose-only grants the
/// import pipeline can't parse (see ticket #66), and for genuine homebrew/DM
/// grants. `available` must already exclude anything granted, chosen, or
/// already added as custom, so re-adding the same value is impossible.
pub fn custom_proficiency_section(
    heading: &'static str,
    entries: RwSignal<Vec<String>>,
    available: impl Fn() -> Vec<String> + Send + Sync + 'static,
    label_fn: impl Fn(&str) -> String + Copy + Send + Sync + 'static,
) -> impl IntoView {
    view! {
        <div class="flex flex-col gap-2 mt-2">
            <h3 class="font-mono text-[0.7rem] tracking-[0.15em] uppercase text-primary/80">
                {heading}
            </h3>
            <div class="flex flex-wrap gap-2">
                {move || {
                    entries
                        .get()
                        .into_iter()
                        .map(|value| {
                            let label = label_fn(&value);
                            let remove_value = value.clone();
                            view! {
                                <span class="badge badge-outline gap-1">
                                    {label}
                                    <span class="opacity-60 text-xs">"(Custom)"</span>
                                    <button
                                        type="button"
                                        class="opacity-70 hover:opacity-100"
                                        aria-label="Remove"
                                        on:click=move |_| {
                                            entries.update(|v| v.retain(|entry| entry != &remove_value));
                                        }
                                    >
                                        "×"
                                    </button>
                                </span>
                            }
                        })
                        .collect_view()
                }}
            </div>
            {
                // A controlled `value` so the select reliably snaps back to
                // the "+ Custom" placeholder after each add — left
                // uncontrolled, the browser instead keeps whatever option
                // slid into the just-picked one's now-vacant list position.
                let picker_value = RwSignal::new(String::new());
                view! {
                    <select
                        class="select select-bordered select-sm w-64"
                        prop:value=move || picker_value.get()
                        on:change=move |ev| {
                            let value = event_target_value(&ev);
                            if !value.is_empty() {
                                entries.update(|v| v.push(value));
                            }
                            picker_value.set(String::new());
                        }
                    >
                        <option value="">"+ Custom"</option>
                        {move || {
                            available()
                                .into_iter()
                                .map(|opt| {
                                    let value = opt.clone();
                                    let label = label_fn(&opt);
                                    view! { <option value=value>{label}</option> }
                                })
                                .collect_view()
                        }}
                    </select>
                }
            }
        </div>
    }
}
