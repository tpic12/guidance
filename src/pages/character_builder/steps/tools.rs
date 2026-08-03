use crate::models::character::{ToolChoicePool, ToolSlots};
use crate::models::proficiency::{contains_ignore_case, title_case};
use leptos::prelude::*;

pub fn tools_step(
    tool_inputs: Memo<ToolSlots>,
    tool_choices: RwSignal<Vec<Option<String>>>,
    tool_proficiency_sources: Memo<Vec<(String, Vec<String>)>>,
    custom_tool_choices: RwSignal<Vec<Option<String>>>,
    all_tool_names: impl Fn() -> Vec<String> + Send + Sync + Copy + 'static,
) -> impl IntoView {
    view! {
        <h2 class="card-title">"Tool Proficiencies"</h2>
        {move || {
            let rows = tool_proficiency_sources.get();
            (!rows.is_empty())
                .then(|| {
                    view! {
                        <div class="card bg-base-200 border border-base-300">
                            <div class="card-body py-3 gap-2">
                                <h3 class="font-mono text-[0.7rem] tracking-[0.15em] uppercase text-primary/80 pb-1 border-b border-base-300">
                                    "Current Proficiencies"
                                </h3>
                                <div class="flex flex-wrap gap-2">
                                    {rows
                                        .into_iter()
                                        .map(|(tool, sources)| {
                                            let label = title_case(&tool);
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
            (tool_inputs.get().choice_pools.is_empty())
                .then(|| {
                    view! {
                        <p class="text-sm opacity-70">
                            "No tool choices to make — this class and background only grant fixed tool proficiencies."
                        </p>
                    }
                })
        }}
        // Keyed on slot index — see `skills_step`'s identical `For` for why.
        <For
            each=move || tool_inputs.get().choice_pools.into_iter().enumerate()
            key=|(slot, _)| *slot
            children=move |(slot, pool)| {
                tool_slot(slot, pool, tool_inputs, tool_choices, custom_tool_choices)
            }
        />
        // Custom rows are keyed on slot index too — see `skills_step`'s
        // identical `For` for why.
        <For
            each=move || custom_tool_choices.get().into_iter().enumerate()
            key=|(slot, _)| *slot
            children=move |(slot, _)| {
                custom_tool_slot(slot, tool_inputs, tool_choices, custom_tool_choices, all_tool_names)
            }
        />
        <button
            type="button"
            class="btn btn-outline btn-sm w-fit"
            on:click=move |_| custom_tool_choices.update(|choices| choices.push(None))
        >
            "+ Add Custom"
        </button>
    }
}

fn tool_slot(
    slot: usize,
    pool: ToolChoicePool,
    tool_inputs: Memo<ToolSlots>,
    tool_choices: RwSignal<Vec<Option<String>>>,
    custom_tool_choices: RwSignal<Vec<Option<String>>>,
) -> impl IntoView {
    let choice = move || tool_choices.get().get(slot).cloned().flatten();
    let set_choice = move |value: Option<String>| {
        tool_choices.update(|choices| {
            if let Some(entry) = choices.get_mut(slot) {
                *entry = value;
            }
        });
    };
    // Disabled once picked in a sibling slot, already granted outright by
    // class/background, or already added as a custom proficiency — picking
    // it here would just waste the slot on a proficiency the character
    // already has.
    let is_disabled = move |opt: &str| {
        contains_ignore_case(&tool_inputs.get().fixed, opt)
            || custom_tool_choices.get().iter().any(|custom| custom.as_deref() == Some(opt))
            || tool_choices
                .get()
                .iter()
                .enumerate()
                .any(|(i, picked)| i != slot && picked.as_deref() == Some(opt))
    };
    let heading = format!("{}: tool choice", pool.source);

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

// Same look as `tool_slot` (a "Custom: tool choice" row is otherwise
// indistinguishable from a granted one), plus a remove control since these
// are directly player-added rather than derived from a grant.
fn custom_tool_slot(
    slot: usize,
    tool_inputs: Memo<ToolSlots>,
    tool_choices: RwSignal<Vec<Option<String>>>,
    custom_tool_choices: RwSignal<Vec<Option<String>>>,
    all_tool_names: impl Fn() -> Vec<String> + Send + Sync + 'static,
) -> impl IntoView {
    let choice = move || custom_tool_choices.get().get(slot).cloned().flatten();
    let set_choice = move |value: Option<String>| {
        custom_tool_choices.update(|choices| {
            if let Some(entry) = choices.get_mut(slot) {
                *entry = value;
            }
        });
    };
    // Disabled once picked in a sibling slot (granted or custom), or already
    // granted outright by class/background — same reasoning as `tool_slot`.
    let is_disabled = move |opt: &str| {
        contains_ignore_case(&tool_inputs.get().fixed, opt)
            || tool_choices.get().iter().any(|picked| picked.as_deref() == Some(opt))
            || custom_tool_choices
                .get()
                .iter()
                .enumerate()
                .any(|(i, picked)| i != slot && picked.as_deref() == Some(opt))
    };
    let remove = move |_| {
        custom_tool_choices.update(|choices| {
            if slot < choices.len() {
                choices.remove(slot);
            }
        });
    };

    view! {
        <div class="flex flex-col gap-1 p-3 border border-base-300 rounded-box">
            <div class="flex items-center justify-between">
                <span class="font-semibold text-sm">"Custom: tool choice"</span>
                <button type="button" class="btn btn-ghost btn-xs" on:click=remove>
                    "Remove"
                </button>
            </div>
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
                    all_tool_names()
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
                        .collect_view()
                }}
            </select>
        </div>
    }
}
