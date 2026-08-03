use crate::models::character::{
    ability_modifier, final_abilities, multiclass_prereq_violations, proficiency_bonus, resolved_hp_max_multiclass,
    total_level, Character, HpMethod,
};
use crate::models::class::Class;
use crate::models::species::Species;
use leptos::prelude::*;
use std::collections::HashMap;

use super::super::{ResolvedClassEntry, STEPS};

pub struct ReviewSummary {
    pub subtitle: String,
    pub skill_choices: Vec<String>,
    pub expertise_choices: Vec<String>,
    pub language_choices: Vec<String>,
    pub tool_choices: Vec<String>,
    pub asi_lines: Vec<String>,
    pub cantrips: Vec<String>,
    pub spells: Vec<String>,
    pub granted_spells: Vec<String>,
    pub optional_features: Vec<String>,
}

pub fn review_step(
    step: RwSignal<usize>,
    step_complete: impl Fn(usize) -> bool + Copy + Send + Sync + 'static,
    step_lock_reason: impl Fn(usize) -> Option<&'static str> + Copy + Send + Sync + 'static,
    build_character: impl Fn() -> Character + Copy + Send + Sync + 'static,
    resolved_entries: Memo<Vec<ResolvedClassEntry>>,
    species_detail: Resource<Result<Option<Species>, ServerFnError>>,
    review_summary: impl Fn() -> ReviewSummary + Copy + Send + Sync + 'static,
    hp_method: RwSignal<HpMethod>,
    hp_rolls: RwSignal<Vec<u8>>,
    hp_manual: RwSignal<Option<i32>>,
    saving: Memo<bool>,
    on_save: impl Fn(Character) + Copy + Send + Sync + 'static,
) -> impl IntoView {
    view! {
        <h2 class="card-title">"Review & Save"</h2>
        {move || {
            let character = build_character();
            let entries = resolved_entries.get();
            let species = species_detail.get().and_then(|result| result.ok()).flatten();
            let finals = final_abilities(&character, species.as_ref());
            let con_mod = ability_modifier(finals[2].1);
            let dex_mod = ability_modifier(finals[1].1);
            let hit_die = entries.first().map(|e| e.detail.class.hit_die).unwrap_or(8);
            let hit_dice: HashMap<String, u8> = entries
                .iter()
                .map(|e| (e.entry.class_id.clone(), e.detail.class.hit_die))
                .collect();
            let hp = (!hit_dice.is_empty())
                .then(|| resolved_hp_max_multiclass(&character, &hit_dice, con_mod));
            let slot_hit_dice: Vec<u8> = entries
                .iter()
                .enumerate()
                .flat_map(|(index, e)| {
                    let levels = if index == 0 {
                        e.entry.level.saturating_sub(1)
                    } else {
                        e.entry.level
                    };
                    std::iter::repeat(e.detail.class.hit_die).take(levels as usize)
                })
                .collect();
            let class_lookup: HashMap<String, Class> = entries
                .iter()
                .map(|e| (e.entry.class_id.clone(), e.detail.class.clone()))
                .collect();
            let prereq_violations = multiclass_prereq_violations(
                &character,
                &class_lookup,
                species.as_ref(),
            );
            let validation = character
                .validate()
                .and_then(|()| {
                    if prereq_violations.is_empty() {
                        Ok(())
                    } else {
                        Err(prereq_violations.join("; "))
                    }
                });
            let summary = review_summary();
            let incomplete_steps: Vec<(usize, &'static str)> = (0..STEPS.len() - 1)
                .filter(|&i| step_lock_reason(i).is_none() && !step_complete(i))
                .map(|i| (i, STEPS[i]))
                .collect();
            // One hit die per level, in the same order save_character
            // re-derives server-side: primary class first (its own level 1
            // is never rolled), then every later class's own levels.
            // Every applicable-but-unfinished step, in step order — locked
            // steps (nothing to pick yet) don't count against completeness.

            view! {
                <div class="flex flex-col gap-1">
                    <h3 class="font-display text-lg font-semibold">{character.name.clone()}</h3>
                    <p class="text-sm opacity-70">{summary.subtitle.clone()}</p>
                </div>
                <div class="stat-seal grid grid-cols-3 sm:grid-cols-6 gap-2 max-w-xl p-3 border border-base-300 rounded-box bg-base-200/50">
                    {finals
                        .map(|(code, score)| {
                            view! {
                                <div class="text-center">
                                    <div class="font-mono text-[0.65rem] tracking-[0.2em] text-primary/80 uppercase">
                                        {code}
                                    </div>
                                    <div class="font-mono text-2xl font-medium leading-tight">
                                        {score.to_string()}
                                    </div>
                                    <div class="font-mono text-xs opacity-60">
                                        {format!("{:+}", ability_modifier(score))}
                                    </div>
                                </div>
                            }
                        })}
                </div>
                <div class="flex flex-wrap gap-4 text-sm font-mono">
                    <span>
                        <span class="font-sans font-semibold">"HP: "</span>
                        {hp.map(|hp| hp.to_string()).unwrap_or_else(|| "—".to_string())}
                    </span>
                    <span>
                        <span class="font-sans font-semibold">"AC: "</span>
                        {format!("{}", 10 + dex_mod as i32)}
                    </span>
                    <span>
                        <span class="font-sans font-semibold">"Proficiency: "</span>
                        {format!("+{}", proficiency_bonus(total_level(&character.classes)))}
                    </span>
                </div>
                <div class="card bg-base-200 p-4 flex flex-col gap-2 max-w-xl">
                    <h4 class="font-semibold text-sm">"Hit Points"</h4>
                    <div role="tablist" class="tabs tabs-box w-fit">
                        {[
                            (HpMethod::Average, "Average"),
                            (HpMethod::Rolled, "Rolled"),
                            (HpMethod::Manual, "Manual"),
                        ]
                            .map(|(method, label)| {
                                view! {
                                    <a
                                        role="tab"
                                        class=move || {
                                            if hp_method.get() == method {
                                                "tab tab-active"
                                            } else {
                                                "tab"
                                            }
                                        }
                                        on:click=move |_| hp_method.set(method)
                                    >
                                        {label}
                                    </a>
                                }
                            })}
                    </div>
                    {match hp_method.get() {
                        HpMethod::Average => {
                            view! {
                                <p class="text-sm opacity-70">
                                    "Uses each level's own class's average hit die (rounded up), plus your Constitution modifier."
                                </p>
                            }
                                .into_any()
                        }
                        HpMethod::Rolled => {
                            let level1 = hit_die as i32 + con_mod as i32;
                            view! {
                                <div class="flex flex-col gap-2">
                                    <p class="text-xs opacity-70">
                                        {format!(
                                            "Level 1: {level1} (max d{hit_die} + Constitution modifier)",
                                        )}
                                    </p>
                                    <div class="flex flex-wrap gap-3">
                                        {(2..=total_level(&character.classes))
                                            .zip(slot_hit_dice.iter().copied())
                                            .enumerate()
                                            .map(|(idx, (lvl, slot_die))| {
                                                view! {
                                                    <label class="flex flex-col items-start gap-1 text-xs">
                                                        {format!("Level {lvl} · d{slot_die}")}
                                                        <input
                                                            type="number"
                                                            class="input input-bordered input-sm w-16"
                                                            min="1"
                                                            max=slot_die.to_string()
                                                            prop:value=move || {
                                                                hp_rolls.get().get(idx).copied().unwrap_or(0).to_string()
                                                            }
                                                            on:input=move |ev| {
                                                                if let Ok(value) = event_target_value(&ev).parse::<u8>() {
                                                                    hp_rolls
                                                                        .update(|rolls| {
                                                                            if let Some(slot) = rolls.get_mut(idx) {
                                                                                *slot = value.clamp(1, slot_die);
                                                                            }
                                                                        });
                                                                }
                                                            }
                                                        />
                                                    </label>
                                                }
                                            })
                                            .collect_view()}
                                    </div>
                                </div>
                            }
                                .into_any()
                        }
                        HpMethod::Manual => {
                            view! {
                                <input
                                    type="number"
                                    class="input input-bordered input-sm w-24"
                                    min="1"
                                    prop:value=move || {
                                        hp_manual.get().map(|hp| hp.to_string()).unwrap_or_default()
                                    }
                                    on:input=move |ev| {
                                        let raw = event_target_value(&ev);
                                        hp_manual.set(raw.parse::<i32>().ok().filter(|hp| *hp > 0));
                                    }
                                />
                            }
                                .into_any()
                        }
                    }}
                </div>
                <div class="flex flex-col gap-3 max-w-xl">
                    {(!summary.skill_choices.is_empty() || !summary.expertise_choices.is_empty())
                        .then(|| {
                            view! {
                                <div class="card bg-base-200 p-4 flex flex-col gap-1">
                                    <h4 class="font-semibold text-sm">"Skills"</h4>
                                    {(!summary.skill_choices.is_empty())
                                        .then(|| {
                                            view! {
                                                <p class="text-sm">{summary.skill_choices.join(", ")}</p>
                                            }
                                        })}
                                    {(!summary.expertise_choices.is_empty())
                                        .then(|| {
                                            view! {
                                                <p class="text-sm opacity-70">
                                                    {format!(
                                                        "Expertise: {}",
                                                        summary.expertise_choices.join(", "),
                                                    )}
                                                </p>
                                            }
                                        })}
                                </div>
                            }
                        })}
                    {(!summary.language_choices.is_empty() || !summary.tool_choices.is_empty())
                        .then(|| {
                            view! {
                                <div class="card bg-base-200 p-4 flex flex-col gap-1">
                                    <h4 class="font-semibold text-sm">"Languages & Tools"</h4>
                                    {(!summary.language_choices.is_empty())
                                        .then(|| {
                                            view! {
                                                <p class="text-sm">
                                                    {format!("Languages: {}", summary.language_choices.join(", "))}
                                                </p>
                                            }
                                        })}
                                    {(!summary.tool_choices.is_empty())
                                        .then(|| {
                                            view! {
                                                <p class="text-sm">
                                                    {format!("Tools: {}", summary.tool_choices.join(", "))}
                                                </p>
                                            }
                                        })}
                                </div>
                            }
                        })}
                    {(!summary.cantrips.is_empty() || !summary.spells.is_empty()
                        || !summary.granted_spells.is_empty())
                        .then(|| {
                            view! {
                                <div class="card bg-base-200 p-4 flex flex-col gap-1">
                                    <h4 class="font-semibold text-sm">"Spells"</h4>
                                    {(!summary.cantrips.is_empty())
                                        .then(|| {
                                            view! {
                                                <p class="text-sm">
                                                    {format!("Cantrips: {}", summary.cantrips.join(", "))}
                                                </p>
                                            }
                                        })}
                                    {(!summary.spells.is_empty())
                                        .then(|| {
                                            view! {
                                                <p class="text-sm">
                                                    {format!("Known/Prepared: {}", summary.spells.join(", "))}
                                                </p>
                                            }
                                        })}
                                    {(!summary.granted_spells.is_empty())
                                        .then(|| {
                                            view! {
                                                <p class="text-sm opacity-70">
                                                    {format!(
                                                        "Granted (always prepared): {}",
                                                        summary.granted_spells.join(", "),
                                                    )}
                                                </p>
                                            }
                                        })}
                                </div>
                            }
                        })}
                    {(!summary.optional_features.is_empty())
                        .then(|| {
                            view! {
                                <div class="card bg-base-200 p-4 flex flex-col gap-1">
                                    <h4 class="font-semibold text-sm">"Optional Features"</h4>
                                    <p class="text-sm">{summary.optional_features.join(", ")}</p>
                                </div>
                            }
                        })}
                    {(!summary.asi_lines.is_empty())
                        .then(|| {
                            view! {
                                <div class="card bg-base-200 p-4 flex flex-col gap-1">
                                    <h4 class="font-semibold text-sm">
                                        "Ability Score Improvements"
                                    </h4>
                                    {summary
                                        .asi_lines
                                        .iter()
                                        .map(|line| view! { <p class="text-sm">{line.clone()}</p> })
                                        .collect_view()}
                                </div>
                            }
                        })}
                </div>
                {(!incomplete_steps.is_empty())
                    .then(|| {
                        view! {
                            <div class="alert alert-warning flex flex-col items-start gap-2 max-w-xl">
                                <span class="font-semibold text-sm">"Still needs:"</span>
                                <div class="flex flex-wrap gap-2">
                                    {incomplete_steps
                                        .iter()
                                        .map(|&(index, title)| {
                                            view! {
                                                <button
                                                    type="button"
                                                    class="btn btn-xs btn-outline"
                                                    on:click=move |_| step.set(index)
                                                >
                                                    {title}
                                                </button>
                                            }
                                        })
                                        .collect_view()}
                                </div>
                            </div>
                        }
                    })}
                {validation
                    .as_ref()
                    .err()
                    .map(|err| view! { <p class="text-warning text-sm">{err.clone()}</p> })}
                <button
                    type="button"
                    class="btn btn-primary w-fit"
                    disabled=validation.is_err() || saving.get() || !incomplete_steps.is_empty()
                    on:click=move |_| on_save(build_character())
                >
                    {if saving.get() { "Saving..." } else { "Save Character" }}
                </button>
            }
        }}
    }
}
