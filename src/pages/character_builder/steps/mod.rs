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

/// One "pick a proficiency" row — a heading, a `<select>` of `options`, and
/// (for player-added custom rows) a "Remove" control. Shared by the Skills/
/// Languages/Tools steps' granted-choice slots, their custom-proficiency
/// slots, and the Skills step's expertise slots — all six/seven are the same
/// shape, differing only in heading text, option source, label formatting,
/// and disabled/removable behavior.
pub fn proficiency_slot(
    heading: String,
    choice: impl Fn() -> Option<String> + Copy + Send + Sync + 'static,
    set_choice: impl Fn(Option<String>) + Send + Sync + 'static,
    options: impl Fn() -> Vec<String> + Send + Sync + 'static,
    is_disabled: impl Fn(String) -> bool + Copy + Send + Sync + 'static,
    label_fn: impl Fn(&str) -> String + Copy + Send + Sync + 'static,
    on_remove: Option<Callback<()>>,
) -> impl IntoView {
    view! {
        <div class="flex flex-col gap-1 p-3 border border-base-300 rounded-box">
            {match on_remove {
                Some(remove) => {
                    view! {
                        <div class="flex items-center justify-between">
                            <span class="font-semibold text-sm">{heading.clone()}</span>
                            <button
                                type="button"
                                class="btn btn-ghost btn-xs"
                                on:click=move |_| remove.run(())
                            >
                                "Remove"
                            </button>
                        </div>
                    }
                        .into_any()
                }
                None => view! { <span class="font-semibold text-sm">{heading.clone()}</span> }.into_any(),
            }}
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
                    options()
                        .into_iter()
                        .map(|opt| {
                            let value = opt.clone();
                            let label = label_fn(&opt);
                            let selected_opt = opt.clone();
                            let disabled_opt = opt.clone();
                            view! {
                                <option
                                    value=value
                                    selected=move || choice().as_deref() == Some(selected_opt.as_str())
                                    disabled=move || is_disabled(disabled_opt.clone())
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
