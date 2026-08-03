use crate::models::character::{LanguageChoicePool, LanguageSlots};
use crate::models::proficiency::{contains_ignore_case, title_case};
use leptos::prelude::*;

pub fn languages_step(
    language_inputs: Memo<LanguageSlots>,
    language_choices: RwSignal<Vec<Option<String>>>,
    language_proficiency_sources: Memo<Vec<(String, Vec<String>)>>,
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
            children=move |(slot, pool)| language_slot(slot, pool, language_inputs, language_choices)
        />
    }
}

fn language_slot(
    slot: usize,
    pool: LanguageChoicePool,
    language_inputs: Memo<LanguageSlots>,
    language_choices: RwSignal<Vec<Option<String>>>,
) -> impl IntoView {
    let choice = move || language_choices.get().get(slot).cloned().flatten();
    let set_choice = move |value: Option<String>| {
        language_choices.update(|choices| {
            if let Some(entry) = choices.get_mut(slot) {
                *entry = value;
            }
        });
    };
    // Disabled once picked in a sibling slot, or once already granted
    // outright by background/species — picking it here would just waste the
    // slot on a language the character already knows.
    let is_disabled = move |opt: &str| {
        contains_ignore_case(&language_inputs.get().fixed, opt)
            || language_choices
                .get()
                .iter()
                .enumerate()
                .any(|(i, picked)| i != slot && picked.as_deref() == Some(opt))
    };
    let heading = format!("{}: language choice", pool.source);

    view! {
        <div class="flex flex-col gap-1 p-3 border border-base-300 rounded-box">
            <span class="font-semibold text-sm">{heading}</span>
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
                    .options
                    .into_iter()
                    .map(|opt| {
                        let value = opt.clone();
                        let label = title_case(&opt);
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
