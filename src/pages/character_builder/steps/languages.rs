use crate::models::character::{LanguageChoicePool, LanguageSlots};
use crate::models::proficiency::{contains_ignore_case, title_case};
use leptos::prelude::*;

use super::proficiency_slot;

pub fn languages_step(
    language_inputs: Memo<LanguageSlots>,
    language_choices: RwSignal<Vec<Option<String>>>,
    language_proficiency_sources: Memo<Vec<(String, Vec<String>)>>,
    custom_language_choices: RwSignal<Vec<Option<String>>>,
    all_language_names: impl Fn() -> Vec<String> + Send + Sync + Copy + 'static,
) -> impl IntoView {
    view! {
        <h2 class="card-title">"Languages"</h2>
        {move || {
            let rows = language_proficiency_sources.get();
            (!rows.is_empty())
                .then(|| {
                    view! {
                        <div class="card bg-base-200 border border-base-300">
                            <div class="card-body py-3 gap-2">
                                <h3 class="font-mono text-[0.7rem] tracking-[0.15em] uppercase text-primary/80 pb-1 border-b border-base-300">
                                    "Current Languages"
                                </h3>
                                <div class="flex flex-wrap gap-2">
                                    {rows
                                        .into_iter()
                                        .map(|(language, sources)| {
                                            let label = title_case(&language);
                                            let source_text = sources.join(", ");
                                            view! {
                                                <span
                                                    class="badge badge-outline gap-1"
                                                    title=source_text.clone()
                                                >
                                                    {label}
                                                    <span class="opacity-60 text-xs">
                                                        {format!("({source_text})")}
                                                    </span>
                                                </span>
                                            }
                                        })
                                        .collect_view()}
                                </div>
                            </div>
                        </div>
                    }
                })
        }}
        {move || {
            (language_inputs.get().choice_pools.is_empty())
                .then(|| {
                    view! {
                        <p class="text-sm opacity-70">
                            "No language choices to make — this background and species only grant fixed languages."
                        </p>
                    }
                })
        }}
        // Keyed on slot index — see `skills_step`'s identical `For` for why.
        <For
            each=move || language_inputs.get().choice_pools.into_iter().enumerate()
            key=|(slot, _)| *slot
            children=move |(slot, pool)| {
                language_slot(slot, pool, language_inputs, language_choices, custom_language_choices)
            }
        />
        // Custom rows are keyed on slot index too — see `skills_step`'s
        // identical `For` for why.
        <For
            each=move || custom_language_choices.get().into_iter().enumerate()
            key=|(slot, _)| *slot
            children=move |(slot, _)| {
                custom_language_slot(slot, language_inputs, language_choices, custom_language_choices, all_language_names)
            }
        />
        <button
            type="button"
            class="btn btn-outline btn-sm w-fit"
            on:click=move |_| custom_language_choices.update(|choices| choices.push(None))
        >
            "+ Add Custom"
        </button>
    }
}

fn language_slot(
    slot: usize,
    pool: LanguageChoicePool,
    language_inputs: Memo<LanguageSlots>,
    language_choices: RwSignal<Vec<Option<String>>>,
    custom_language_choices: RwSignal<Vec<Option<String>>>,
) -> impl IntoView {
    let heading = format!("{}: language choice", pool.source);
    let options = pool.options;
    // Disabled once picked in a sibling slot, already granted outright by
    // background/species, or already added as a custom proficiency —
    // picking it here would just waste the slot on a language the character
    // already knows.
    let is_disabled = move |opt: String| {
        contains_ignore_case(&language_inputs.get().fixed, &opt)
            || custom_language_choices.get().iter().any(|custom| custom.as_deref() == Some(opt.as_str()))
            || language_choices
                .get()
                .iter()
                .enumerate()
                .any(|(i, picked)| i != slot && picked.as_deref() == Some(opt.as_str()))
    };

    proficiency_slot(
        heading,
        move || language_choices.get().get(slot).cloned().flatten(),
        move |value| {
            language_choices.update(|choices| {
                if let Some(entry) = choices.get_mut(slot) {
                    *entry = value;
                }
            });
        },
        move || options.clone(),
        is_disabled,
        title_case,
        None,
    )
}

// Same shape as `language_slot` (a "Custom: language choice" row is
// otherwise indistinguishable from a granted one), plus a remove control
// since these are directly player-added rather than derived from a grant.
fn custom_language_slot(
    slot: usize,
    language_inputs: Memo<LanguageSlots>,
    language_choices: RwSignal<Vec<Option<String>>>,
    custom_language_choices: RwSignal<Vec<Option<String>>>,
    all_language_names: impl Fn() -> Vec<String> + Send + Sync + 'static,
) -> impl IntoView {
    // Disabled once picked in a sibling slot (granted or custom), or already
    // granted outright by background/species — same reasoning as
    // `language_slot`.
    let is_disabled = move |opt: String| {
        contains_ignore_case(&language_inputs.get().fixed, &opt)
            || language_choices.get().iter().any(|picked| picked.as_deref() == Some(opt.as_str()))
            || custom_language_choices
                .get()
                .iter()
                .enumerate()
                .any(|(i, picked)| i != slot && picked.as_deref() == Some(opt.as_str()))
    };
    let on_remove = Callback::new(move |()| {
        custom_language_choices.update(|choices| {
            if slot < choices.len() {
                choices.remove(slot);
            }
        });
    });

    proficiency_slot(
        "Custom: language choice".to_string(),
        move || custom_language_choices.get().get(slot).cloned().flatten(),
        move |value| {
            custom_language_choices.update(|choices| {
                if let Some(entry) = choices.get_mut(slot) {
                    *entry = value;
                }
            });
        },
        all_language_names,
        is_disabled,
        title_case,
        Some(on_remove),
    )
}
