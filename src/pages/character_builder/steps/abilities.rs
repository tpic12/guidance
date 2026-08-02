use crate::models::character::{
    ability_modifier, point_buy_cost, resolve_ability_bonus, AbilityBonusSource, AbilityMethod, AbilityScores,
    ABILITY_CODES, POINT_BUY_BUDGET, STANDARD_ARRAY,
};
use crate::models::class::ability_label;
use crate::models::species::{is_free_ability_choice, AbilityBonusGrant, Species};
use leptos::prelude::*;

pub fn abilities_step(
    ability_method: RwSignal<AbilityMethod>,
    abilities: RwSignal<AbilityScores>,
    species_detail: Resource<Result<Option<Species>, ServerFnError>>,
    species_ability_choices: RwSignal<Vec<String>>,
    ability_bonus_source: RwSignal<AbilityBonusSource>,
    custom_ability_bonus_choices: RwSignal<Vec<String>>,
) -> impl IntoView {
    let species = move || species_detail.get().and_then(|result| result.ok()).flatten();
    // The species' one Choose grant, if it has one — same at-most-one
    // assumption as `Character::species_ability_choices`.
    let choose_grant = move || {
        species().and_then(|species| {
            species.ability_bonuses.into_iter().find_map(|grant| match grant {
                AbilityBonusGrant::Choose { count, amount, from } => Some((count, amount, from)),
                AbilityBonusGrant::Fixed { .. } => None,
            })
        })
    };
    // Base scores with the species/custom bonus folded in (but not yet ASI,
    // which isn't chosen until a later step) — the same `resolve_ability_bonus`
    // `final_abilities` uses, so this preview can never drift from the real
    // derivation.
    let bonused_scores = move || {
        resolve_ability_bonus(
            abilities.get(),
            species().as_ref(),
            ability_bonus_source.get(),
            &species_ability_choices.get(),
            &custom_ability_bonus_choices.get(),
        )
    };
    let switch_bonus_source = move |source: AbilityBonusSource| {
        if ability_bonus_source.get() == source {
            return;
        }
        ability_bonus_source.set(source);
        match source {
            AbilityBonusSource::Species => custom_ability_bonus_choices.update(Vec::clear),
            AbilityBonusSource::Custom => {
                species_ability_choices.update(Vec::clear);
                custom_ability_bonus_choices.set(vec!["str".to_string(), "dex".to_string()]);
            }
        }
    };
    let switch_method = move |method: AbilityMethod| {
        if ability_method.get() == method {
            return;
        }
        ability_method.set(method);
        abilities.set(match method {
            AbilityMethod::StandardArray => AbilityScores {
                strength: 15,
                dexterity: 14,
                constitution: 13,
                intelligence: 12,
                wisdom: 10,
                charisma: 8,
            },
            AbilityMethod::PointBuy => AbilityScores {
                strength: 8,
                dexterity: 8,
                constitution: 8,
                intelligence: 8,
                wisdom: 8,
                charisma: 8,
            },
            AbilityMethod::Manual => AbilityScores::default(),
        });
    };

    let species_hint = move || species().and_then(|species| species.ability);

    let spent = move || {
        ABILITY_CODES
            .iter()
            .map(|code| point_buy_cost(abilities.get().get(code)).unwrap_or(0) as u32)
            .sum::<u32>()
    };

    // Standard Array assignment: rather than six independent <select>s that
    // silently swap another ability's value out from under the player when
    // a score already in use is picked, the player explicitly "arms" a
    // score from this pool, then clicks the ability to drop it into — the
    // same underlying swap, but driven by two deliberate clicks instead of
    // a surprising side effect of one dropdown.
    let armed_value = RwSignal::new(None::<u8>);

    view! {
        <h2 class="card-title">"Ability Scores"</h2>
        <div role="tablist" class="tabs tabs-box w-fit">
            {[
                (AbilityMethod::StandardArray, "Standard Array"),
                (AbilityMethod::PointBuy, "Point Buy"),
                (AbilityMethod::Manual, "Manual"),
            ]
                .map(|(method, label)| {
                    view! {
                        <a
                            role="tab"
                            class=move || {
                                if ability_method.get() == method {
                                    "tab tab-active"
                                } else {
                                    "tab"
                                }
                            }
                            on:click=move |_| switch_method(method)
                        >
                            {label}
                        </a>
                    }
                })}
        </div>

        <div role="tablist" class="tabs tabs-box w-fit">
            {[
                (AbilityBonusSource::Species, "Species bonus"),
                (AbilityBonusSource::Custom, "Custom bonus"),
            ]
                .map(|(source, label)| {
                    view! {
                        <a
                            role="tab"
                            class=move || {
                                if ability_bonus_source.get() == source {
                                    "tab tab-active"
                                } else {
                                    "tab"
                                }
                            }
                            on:click=move |_| switch_bonus_source(source)
                        >
                            {label}
                        </a>
                    }
                })}
        </div>

        {move || {
            if ability_bonus_source.get() != AbilityBonusSource::Species {
                return None;
            }
            species_hint()
                .map(|hint| {
                    view! { <p class="text-sm opacity-70">{format!("Species bonus: {hint}")}</p> }
                })
        }}

        {move || {
            if ability_bonus_source.get() != AbilityBonusSource::Species {
                return None;
            }
            choose_grant()
                .map(|(count, amount, from)| {
                    let candidates: Vec<&'static str> = ABILITY_CODES
                        .into_iter()
                        .filter(|code| {
                            is_free_ability_choice(&from) || from.contains(&code.to_string())
                        })
                        .collect();
                    view! {
                        <div class="flex flex-col gap-1">
                            <p class="text-sm opacity-70">
                                {format!("Choose {count} to gain {amount:+} each:")}
                            </p>
                            <div class="flex flex-wrap gap-2">
                                {candidates
                                    .into_iter()
                                    .map(|code| {
                                        let selected = move || {
                                            species_ability_choices
                                                .get()
                                                .iter()
                                                .any(|picked| picked == code)
                                        };
                                        view! {
                                            <button
                                                type="button"
                                                class=move || {
                                                    if selected() {
                                                        "btn btn-sm btn-primary"
                                                    } else {
                                                        "btn btn-sm btn-outline"
                                                    }
                                                }
                                                disabled=move || {
                                                    !selected()
                                                        && species_ability_choices.get().len() >= count as usize
                                                }
                                                on:click=move |_| {
                                                    species_ability_choices
                                                        .update(|choices| {
                                                            if let Some(pos) = choices
                                                                .iter()
                                                                .position(|picked| picked == code)
                                                            {
                                                                choices.remove(pos);
                                                            } else {
                                                                choices.push(code.to_string());
                                                            }
                                                        });
                                                }
                                            >
                                                {ability_label(code)}
                                            </button>
                                        }
                                    })
                                    .collect_view()}
                            </div>
                        </div>
                    }
                })
        }}

        {move || {
            if ability_bonus_source.get() != AbilityBonusSource::Custom {
                return None;
            }
            let is_disabled = move |position: usize, code: &'static str| {
                custom_ability_bonus_choices
                    .get()
                    .iter()
                    .enumerate()
                    .any(|(i, picked)| i != position && picked == code)
            };
            let custom_ability_select = move |position: usize, label: &'static str| {
                let codes = custom_ability_bonus_choices.get();
                let current = codes.get(position).cloned().unwrap_or_else(|| "str".to_string());
                // Disabled once picked in the other slot — a repeated code isn't
                // meaningful here, unlike `AsiChoice::Abilities`'s "same twice
                // for +2" (see `apply_custom_species_bonus`'s doc comment).
                view! {
                    <div class="flex flex-col gap-1">
                        <span class="text-xs opacity-70">{label}</span>
                        <select
                            class="select select-bordered select-sm"
                            on:change=move |ev| {
                                custom_ability_bonus_choices
                                    .update(|codes| {
                                        codes.resize(2, "str".to_string());
                                        codes[position] = event_target_value(&ev);
                                    });
                            }
                        >
                            {ABILITY_CODES
                                .map(|code| {
                                    let selected = current.clone();
                                    view! {
                                        <option
                                            value=code
                                            selected=move || selected == code
                                            disabled=move || is_disabled(position, code)
                                        >
                                            {ability_label(code)}
                                        </option>
                                    }
                                })}
                        </select>
                    </div>
                }
            };
            Some(
                view! {
                    <div class="flex flex-col gap-1">
                        <p class="text-sm opacity-70">
                            "Pick two different abilities: the first gains +2, the second +1."
                        </p>
                        <div class="flex flex-wrap gap-2 items-end">
                            {custom_ability_select(0, "+2")} {custom_ability_select(1, "+1")}
                        </div>
                    </div>
                },
            )
        }}

        {move || match ability_method.get() {
            AbilityMethod::StandardArray => {
                view! {
                    <div class="flex flex-col gap-2">
                        <p class="text-sm opacity-70">
                            "Click a score, then click an ability to assign it."
                        </p>
                        <div class="flex flex-wrap gap-2">
                            {STANDARD_ARRAY
                                .map(|value| {
                                    let owner = move || {
                                        ABILITY_CODES
                                            .iter()
                                            .find(|code| abilities.get().get(code) == value)
                                            .map(|code| code.to_uppercase())
                                            .unwrap_or_default()
                                    };
                                    view! {
                                        <button
                                            type="button"
                                            class=move || {
                                                if armed_value.get() == Some(value) {
                                                    "btn btn-sm btn-primary h-auto py-1 flex-col gap-0"
                                                } else {
                                                    "btn btn-sm btn-outline h-auto py-1 flex-col gap-0"
                                                }
                                            }
                                            on:click=move |_| {
                                                armed_value
                                                    .update(|armed| {
                                                        *armed = if *armed == Some(value) {
                                                            None
                                                        } else {
                                                            Some(value)
                                                        };
                                                    })
                                            }
                                        >
                                            <span class="text-base font-semibold">
                                                {value.to_string()}
                                            </span>
                                            <span class="text-[10px] font-normal opacity-70">
                                                {owner}
                                            </span>
                                        </button>
                                    }
                                })}
                        </div>
                    </div>
                }
                    .into_any()
            }
            AbilityMethod::PointBuy => {
                view! {
                    <p class="text-sm">
                        {move || format!("{} of {POINT_BUY_BUDGET} points spent", spent())}
                    </p>
                }
                    .into_any()
            }
            _ => ().into_any(),
        }}

        <div class="grid grid-cols-2 sm:grid-cols-3 gap-3 max-w-xl">
            {ABILITY_CODES
                .map(|code| {
                    let score = move || abilities.get().get(code);
                    let bonus = move || bonused_scores().get(code) as i16 - score() as i16;
                    let modifier = move || {
                        format!("{:+}", ability_modifier(bonused_scores().get(code)))
                    };
                    view! {
                        <div class="flex flex-col gap-1 p-2 border border-base-300 rounded-box">
                            <span class="text-sm font-semibold">
                                {ability_label(code)} " " <span class="opacity-60">{modifier}</span>
                            </span>
                            {move || match ability_method.get() {
                                AbilityMethod::StandardArray => {
                                    view! {
                                        <button
                                            type="button"
                                            class="btn btn-outline btn-sm w-full text-lg font-bold"
                                            disabled=move || {
                                                armed_value.get().is_none_or(|value| value == score())
                                            }
                                            on:click=move |_| {
                                                if let Some(value) = armed_value.get() {
                                                    abilities
                                                        .update(|scores| {
                                                            let current = scores.get(code);
                                                            if let Some(other) = ABILITY_CODES
                                                                .iter()
                                                                .find(|other| scores.get(other) == value)
                                                            {
                                                                scores.set(other, current);
                                                            }
                                                            scores.set(code, value);
                                                        });
                                                    armed_value.set(None);
                                                }
                                            }
                                        >
                                            {move || score().to_string()}
                                        </button>
                                    }
                                        .into_any()
                                }
                                AbilityMethod::PointBuy => {
                                    view! {
                                        <div class="join">
                                            <button
                                                type="button"
                                                class="btn btn-sm join-item"
                                                disabled=move || abilities.get().get(code) <= 8
                                                on:click=move |_| {
                                                    abilities
                                                        .update(|scores| {
                                                            scores.set(code, scores.get(code).saturating_sub(1).max(8))
                                                        })
                                                }
                                            >
                                                "-"
                                            </button>
                                            <span class="btn btn-sm join-item pointer-events-none">
                                                {move || score().to_string()}
                                            </span>
                                            <button
                                                type="button"
                                                class="btn btn-sm join-item"
                                                disabled=move || {
                                                    let scores = abilities.get();
                                                    let next = scores.get(code) + 1;
                                                    match point_buy_cost(next) {
                                                        None => true,
                                                        Some(next_cost) => {
                                                            let current_cost = point_buy_cost(scores.get(code))
                                                                .unwrap_or(0);
                                                            spent() + (next_cost - current_cost) as u32
                                                                > POINT_BUY_BUDGET as u32
                                                        }
                                                    }
                                                }
                                                on:click=move |_| {
                                                    abilities
                                                        .update(|scores| scores.set(code, scores.get(code) + 1))
                                                }
                                            >
                                                "+"
                                            </button>
                                        </div>
                                    }
                                        .into_any()
                                }
                                AbilityMethod::Manual => {
                                    view! {
                                        <input
                                            type="number"
                                            class="input input-bordered input-sm"
                                            min="3"
                                            max="20"
                                            prop:value=move || score().to_string()
                                            on:input=move |ev| {
                                                if let Ok(value) = event_target_value(&ev).parse::<u8>() {
                                                    abilities
                                                        .update(|scores| scores.set(code, value.clamp(3, 20)));
                                                }
                                            }
                                        />
                                    }
                                        .into_any()
                                }
                            }}
                            {move || {
                                let bonus = bonus();
                                (bonus != 0)
                                    .then(|| {
                                        view! {
                                            <span class="text-xs opacity-70">
                                                {format!("{bonus:+} species")}
                                            </span>
                                        }
                                    })
                            }}
                        </div>
                    }
                })}
        </div>
    }
}
