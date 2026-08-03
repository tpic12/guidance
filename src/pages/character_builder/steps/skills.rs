use crate::models::character::{SkillChoicePool, SkillSlots};
use crate::models::skill::{skill_label, SKILLS};
use leptos::prelude::*;

pub fn skills_step(
    skill_inputs: Memo<SkillSlots>,
    skill_choices: RwSignal<Vec<Option<String>>>,
    expertise_slot_count: Memo<usize>,
    expertise_choices: RwSignal<Vec<Option<String>>>,
    skill_proficiency_sources: Memo<Vec<(String, Vec<String>)>>,
    custom_skill_choices: RwSignal<Vec<Option<String>>>,
) -> impl IntoView {
    view! {
        <h2 class="card-title">"Skill Proficiencies"</h2>
        {move || {
            let rows = skill_proficiency_sources.get();
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
                                        .map(|(skill, sources)| {
                                            let label = skill_label(&skill);
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
            (skill_inputs.get().choice_pools.is_empty())
                .then(|| {
                    view! {
                        <p class="text-sm opacity-70">
                            "No skill choices to make — this class and background only grant fixed proficiencies."
                        </p>
                    }
                })
        }}
        // Keyed on slot index so a `skill_inputs` recompute that leaves the
        // slot count/content unchanged (e.g. once a still-loading resource
        // resolves) patches in place instead of tearing down and rebuilding
        // every <select>, which could otherwise race with an in-flight
        // selection and silently drop it.
        <For
            each=move || skill_inputs.get().choice_pools.into_iter().enumerate()
            key=|(slot, _)| *slot
            children=move |(slot, pool)| {
                skill_slot(slot, pool, skill_inputs, skill_choices, custom_skill_choices)
            }
        />
        // Custom rows are keyed on slot index too — same reasoning as the
        // granted slots above, and it also keeps a slot's own selection
        // stable while an unrelated slot is added/removed elsewhere in the
        // list, since removal always reindexes the tail.
        <For
            each=move || custom_skill_choices.get().into_iter().enumerate()
            key=|(slot, _)| *slot
            children=move |(slot, _)| {
                custom_skill_slot(slot, skill_inputs, skill_choices, custom_skill_choices)
            }
        />
        <button
            type="button"
            class="btn btn-outline btn-sm w-fit"
            on:click=move |_| custom_skill_choices.update(|choices| choices.push(None))
        >
            "+ Add Custom"
        </button>
        {move || {
            (expertise_slot_count.get() > 0)
                .then(|| view! { <h2 class="card-title mt-4">"Expertise"</h2> })
        }}
        <For
            each=move || 0..expertise_slot_count.get()
            key=|slot| *slot
            children=move |slot| expertise_slot(
                slot,
                skill_inputs,
                skill_choices,
                custom_skill_choices,
                expertise_choices,
            )
        />
    }
}

fn skill_slot(
    slot: usize,
    pool: SkillChoicePool,
    skill_inputs: Memo<SkillSlots>,
    skill_choices: RwSignal<Vec<Option<String>>>,
    custom_skill_choices: RwSignal<Vec<Option<String>>>,
) -> impl IntoView {
    let choice = move || skill_choices.get().get(slot).cloned().flatten();
    let set_choice = move |value: Option<String>| {
        skill_choices.update(|choices| {
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
        skill_inputs.get().fixed.iter().any(|fixed| fixed == opt)
            || custom_skill_choices.get().iter().any(|custom| custom.as_deref() == Some(opt))
            || skill_choices
                .get()
                .iter()
                .enumerate()
                .any(|(i, picked)| i != slot && picked.as_deref() == Some(opt))
    };
    let heading = format!("{}: skill choice", pool.source);

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
                        let label = skill_label(&opt);
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

// Same look as `skill_slot` (a "Custom: skill choice" row is otherwise
// indistinguishable from a granted one), plus a remove control since these
// are directly player-added rather than derived from a grant.
fn custom_skill_slot(
    slot: usize,
    skill_inputs: Memo<SkillSlots>,
    skill_choices: RwSignal<Vec<Option<String>>>,
    custom_skill_choices: RwSignal<Vec<Option<String>>>,
) -> impl IntoView {
    let choice = move || custom_skill_choices.get().get(slot).cloned().flatten();
    let set_choice = move |value: Option<String>| {
        custom_skill_choices.update(|choices| {
            if let Some(entry) = choices.get_mut(slot) {
                *entry = value;
            }
        });
    };
    // Disabled once picked in a sibling slot (granted or custom), or already
    // granted outright by class/background — same reasoning as `skill_slot`.
    let is_disabled = move |opt: &str| {
        skill_inputs.get().fixed.iter().any(|fixed| fixed == opt)
            || skill_choices.get().iter().any(|picked| picked.as_deref() == Some(opt))
            || custom_skill_choices
                .get()
                .iter()
                .enumerate()
                .any(|(i, picked)| i != slot && picked.as_deref() == Some(opt))
    };
    let remove = move |_| {
        custom_skill_choices.update(|choices| {
            if slot < choices.len() {
                choices.remove(slot);
            }
        });
    };

    view! {
        <div class="flex flex-col gap-1 p-3 border border-base-300 rounded-box">
            <div class="flex items-center justify-between">
                <span class="font-semibold text-sm">"Custom: skill choice"</span>
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
                {SKILLS
                    .iter()
                    .map(|(name, _)| {
                        let value = name.to_string();
                        let label = skill_label(name);
                        let selected_opt = name.to_string();
                        let disabled_opt = name.to_string();
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

fn expertise_slot(
    slot: usize,
    skill_inputs: Memo<SkillSlots>,
    skill_choices: RwSignal<Vec<Option<String>>>,
    custom_skill_choices: RwSignal<Vec<Option<String>>>,
    expertise_choices: RwSignal<Vec<Option<String>>>,
) -> impl IntoView {
    let choice = move || expertise_choices.get().get(slot).cloned().flatten();
    let set_choice = move |value: Option<String>| {
        expertise_choices.update(|choices| {
            if let Some(entry) = choices.get_mut(slot) {
                *entry = value;
            }
        });
    };
    let proficient_pool = move || {
        let mut pool = skill_inputs.get().fixed;
        pool.extend(skill_choices.get().into_iter().flatten());
        pool.extend(custom_skill_choices.get().into_iter().flatten());
        pool.sort();
        pool.dedup();
        pool
    };
    let is_disabled = move |opt: &str| {
        expertise_choices
            .get()
            .iter()
            .enumerate()
            .any(|(i, picked)| i != slot && picked.as_deref() == Some(opt))
    };

    view! {
        <div class="flex flex-col gap-1 p-3 border border-base-300 rounded-box">
            <span class="font-semibold text-sm">{format!("Expertise choice {}", slot + 1)}</span>
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
                    proficient_pool()
                        .into_iter()
                        .map(|opt| {
                            let value = opt.clone();
                            let label = skill_label(&opt);
                            let selected_opt = opt.clone();
                            let disabled_opt = opt.clone();
                            view! {
                                <option
                                    value=value
                                    selected=move || {
                                        choice().as_deref() == Some(selected_opt.as_str())
                                    }
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
