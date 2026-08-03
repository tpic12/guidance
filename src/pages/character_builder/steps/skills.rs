use crate::models::character::{SkillChoicePool, SkillSlots};
use crate::models::skill::{skill_label, SKILLS};
use leptos::prelude::*;

use super::proficiency_slot;

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
    let heading = format!("{}: skill choice", pool.source);
    let options = pool.options;
    // Disabled once picked in a sibling slot, already granted outright by
    // class/background, or already added as a custom proficiency — picking
    // it here would just waste the slot on a proficiency the character
    // already has.
    let is_disabled = move |opt: String| {
        skill_inputs.get().fixed.iter().any(|fixed| fixed == &opt)
            || custom_skill_choices.get().iter().any(|custom| custom.as_deref() == Some(opt.as_str()))
            || skill_choices
                .get()
                .iter()
                .enumerate()
                .any(|(i, picked)| i != slot && picked.as_deref() == Some(opt.as_str()))
    };

    proficiency_slot(
        heading,
        move || skill_choices.get().get(slot).cloned().flatten(),
        move |value| {
            skill_choices.update(|choices| {
                if let Some(entry) = choices.get_mut(slot) {
                    *entry = value;
                }
            });
        },
        move || options.clone(),
        is_disabled,
        skill_label,
        None,
    )
}

// Same shape as `skill_slot` (a "Custom: skill choice" row is otherwise
// indistinguishable from a granted one), plus a remove control since these
// are directly player-added rather than derived from a grant.
fn custom_skill_slot(
    slot: usize,
    skill_inputs: Memo<SkillSlots>,
    skill_choices: RwSignal<Vec<Option<String>>>,
    custom_skill_choices: RwSignal<Vec<Option<String>>>,
) -> impl IntoView {
    // Disabled once picked in a sibling slot (granted or custom), or already
    // granted outright by class/background — same reasoning as `skill_slot`.
    let is_disabled = move |opt: String| {
        skill_inputs.get().fixed.iter().any(|fixed| fixed == &opt)
            || skill_choices.get().iter().any(|picked| picked.as_deref() == Some(opt.as_str()))
            || custom_skill_choices
                .get()
                .iter()
                .enumerate()
                .any(|(i, picked)| i != slot && picked.as_deref() == Some(opt.as_str()))
    };
    let on_remove = Callback::new(move |()| {
        custom_skill_choices.update(|choices| {
            if slot < choices.len() {
                choices.remove(slot);
            }
        });
    });

    proficiency_slot(
        "Custom: skill choice".to_string(),
        move || custom_skill_choices.get().get(slot).cloned().flatten(),
        move |value| {
            custom_skill_choices.update(|choices| {
                if let Some(entry) = choices.get_mut(slot) {
                    *entry = value;
                }
            });
        },
        || SKILLS.iter().map(|(name, _)| name.to_string()).collect(),
        is_disabled,
        skill_label,
        Some(on_remove),
    )
}

fn expertise_slot(
    slot: usize,
    skill_inputs: Memo<SkillSlots>,
    skill_choices: RwSignal<Vec<Option<String>>>,
    custom_skill_choices: RwSignal<Vec<Option<String>>>,
    expertise_choices: RwSignal<Vec<Option<String>>>,
) -> impl IntoView {
    let proficient_pool = move || {
        let mut pool = skill_inputs.get().fixed;
        pool.extend(skill_choices.get().into_iter().flatten());
        pool.extend(custom_skill_choices.get().into_iter().flatten());
        pool.sort();
        pool.dedup();
        pool
    };
    let is_disabled = move |opt: String| {
        expertise_choices
            .get()
            .iter()
            .enumerate()
            .any(|(i, picked)| i != slot && picked.as_deref() == Some(opt.as_str()))
    };

    proficiency_slot(
        format!("Expertise choice {}", slot + 1),
        move || expertise_choices.get().get(slot).cloned().flatten(),
        move |value| {
            expertise_choices.update(|choices| {
                if let Some(entry) = choices.get_mut(slot) {
                    *entry = value;
                }
            });
        },
        proficient_pool,
        is_disabled,
        skill_label,
        None,
    )
}
