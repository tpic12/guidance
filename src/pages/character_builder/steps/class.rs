use crate::models::character::ClassLevel;
use crate::models::class::{ability_label, ClassDetail};
use leptos::prelude::*;
use std::collections::{HashMap, HashSet};

use super::super::class_band_class;
use crate::models::character::subclass_unlock_level;

pub fn class_step(
    classes: RwSignal<Vec<ClassLevel>>,
    class_list: Resource<Result<Vec<crate::models::class::ClassSummary>, ServerFnError>>,
    class_details: Resource<Result<(Vec<String>, Vec<(String, ClassDetail)>), ServerFnError>>,
    current_final_abilities: impl Fn() -> [(&'static str, u8); 6] + Copy + Send + Sync + 'static,
    enforce_prereqs: RwSignal<bool>,
) -> impl IntoView {
    let details_map = move || -> HashMap<String, ClassDetail> {
        class_details.get().and_then(|result| result.ok()).map(|(_, list)| list.into_iter().collect()).unwrap_or_default()
    };
    let total = move || classes.get().iter().fold(0u8, |acc, c| acc.saturating_add(c.level));
    // Soft, non-blocking hint only — Save/Review is the real gate.
    let unmet_prereq_label = move |prereqs: &[Vec<(String, u8)>], multiclass_active: bool| -> Option<String> {
        if !enforce_prereqs.get() || prereqs.is_empty() || !multiclass_active {
            return None;
        }
        let scores = current_final_abilities();
        let get = |code: &str| scores.iter().find(|(c, _)| *c == code).map(|(_, v)| *v).unwrap_or(0);
        let met = prereqs.iter().any(|group| group.iter().all(|(ability, min)| get(ability) >= *min));
        if met {
            return None;
        }
        let alternatives = prereqs
            .iter()
            .map(|group| {
                group
                    .iter()
                    .map(|(ability, min)| format!("{} {min}", ability_label(ability)))
                    .collect::<Vec<_>>()
                    .join(" and ")
            })
            .collect::<Vec<_>>()
            .join(" or ");
        Some(format!("⚠ Requires {alternatives}"))
    };

    view! {
        <h2 class="card-title">"Classes"</h2>
        <span class="font-mono text-sm opacity-80">
            {move || format!("TOTAL LEVEL {:02} / 20", total())}
        </span>

        <div class="flex flex-col gap-3">
            <For
                each=move || {
                    classes.get().into_iter().enumerate().map(|(i, c)| (c.class_id.clone(), i))
                }
                key=|(class_id, _)| class_id.clone()
                children=move |(row_class_id, _)| {
                    let row_id = row_class_id.clone();
                    let position = move || {
                        classes.get().iter().position(|c| c.class_id == row_id).unwrap_or(0)
                    };
                    let row_id = row_class_id.clone();
                    let current_level = move || {
                        classes
                            .get()
                            .iter()
                            .find(|c| c.class_id == row_id)
                            .map(|c| c.level)
                            .unwrap_or(1)
                    };
                    let row_id = row_class_id.clone();
                    let current_subclass_id = move || {
                        classes
                            .get()
                            .into_iter()
                            .find(|c| c.class_id == row_id)
                            .and_then(|c| c.subclass_id)
                    };
                    let row_id = row_class_id.clone();
                    let detail = move || details_map().get(&row_id).cloned();
                    let dec_id = row_class_id.clone();
                    let dec_click = move |_: leptos::ev::MouseEvent| {
                        classes
                            .update(|list| {
                                if let Some(entry) = list.iter_mut().find(|c| c.class_id == dec_id)
                                {
                                    entry.level = entry.level.saturating_sub(1).max(1);
                                }
                            });
                    };
                    let inc_id = row_class_id.clone();
                    let inc_click = move |_: leptos::ev::MouseEvent| {
                        classes
                            .update(|list| {
                                if let Some(entry) = list.iter_mut().find(|c| c.class_id == inc_id)
                                {
                                    entry.level = (entry.level + 1).min(20);
                                }
                            });
                    };
                    let remove_id = row_class_id.clone();
                    let remove_click = move |_: leptos::ev::MouseEvent| {
                        classes.update(|list| list.retain(|c| c.class_id != remove_id));
                    };
                    let subclass_row_id = row_class_id.clone();
                    let detail_for_name = detail.clone();
                    let detail_for_subclass = detail.clone();
                    let detail_for_prereq_warning = detail.clone();
                    let current_level_for_dec = current_level.clone();
                    let current_level_for_display = current_level.clone();
                    let current_level_for_subclass = current_level.clone();
                    let maxed_out = move || total() >= 20;

                    // A literal `>=` inside a `view!` attribute expression
                    // confuses the macro's tag scanner (it reads the bare `>`
                    // as closing the element early) — named boolean signals
                    // sidestep the parser entirely, same fix as the on:click
                    // handlers above.

                    view! {
                        <div class=move || {
                            format!(
                                "class-band {} flex flex-col gap-2 p-3 border border-base-300 rounded-box bg-base-200/40",
                                class_band_class(position()),
                            )
                        }>
                            <div class="flex items-center gap-3 flex-wrap">
                                <span class="font-display text-lg flex-1 min-w-40">
                                    {move || {
                                        detail_for_name()
                                            .map(|d| d.class.name.clone())
                                            .unwrap_or_else(|| "Loading...".to_string())
                                    }}
                                </span>
                                <div class="join">
                                    <button
                                        type="button"
                                        class="btn btn-square btn-sm join-item"
                                        disabled=move || current_level_for_dec() <= 1
                                        on:click=dec_click
                                    >
                                        "−"
                                    </button>
                                    <span class="btn btn-sm join-item pointer-events-none font-mono">
                                        {move || format!("{:02}", current_level_for_display())}
                                    </span>
                                    <button
                                        type="button"
                                        class="btn btn-square btn-sm join-item"
                                        disabled=maxed_out
                                        on:click=inc_click
                                    >
                                        "+"
                                    </button>
                                </div>
                                <button
                                    type="button"
                                    class="btn btn-ghost btn-xs btn-circle"
                                    title="Remove this class"
                                    on:click=remove_click
                                >
                                    "✕"
                                </button>
                            </div>
                            {move || {
                                let detail = detail_for_prereq_warning()?;
                                let label = unmet_prereq_label(
                                    &detail.class.multiclass_ability_prerequisites,
                                    classes.get().len() >= 2,
                                )?;
                                Some(view! { <p class="text-warning text-xs">{label}</p> })
                            }}
                            {move || {
                                let detail = detail_for_subclass()?;
                                let unlock = subclass_unlock_level(&detail)?;
                                let title = detail.class.subclass_title.clone();
                                let row_id = subclass_row_id.clone();
                                Some(
                                    if unlock <= current_level_for_subclass() {
                                        view! {
                                            <div class="flex flex-col gap-2">
                                                <h3 class="font-semibold text-sm">{title}</h3>
                                                <div class="flex flex-wrap gap-2">
                                                    {detail
                                                        .subclasses
                                                        .iter()
                                                        .map(|sub| {
                                                            let sub_id = sub.subclass.id.clone();
                                                            let selected = sub.subclass.id.clone();
                                                            let label = format!(
                                                                "{} ({})",
                                                                sub.subclass.short_name,
                                                                sub.subclass.source,
                                                            );
                                                            let row_id = row_id.clone();
                                                            let click_row_id = row_id.clone();
                                                            let sub_click = move |_: leptos::ev::MouseEvent| {
                                                                classes
                                                                    .update(|list| {
                                                                        if let Some(entry) = list
                                                                            .iter_mut()
                                                                            .find(|c| c.class_id == click_row_id)
                                                                        {
                                                                            entry.subclass_id = Some(sub_id.clone());
                                                                        }
                                                                    });
                                                            };
                                                            let current_subclass_id = current_subclass_id.clone();
                                                            view! {
                                                                <button
                                                                    type="button"
                                                                    class=move || {
                                                                        if current_subclass_id().as_deref()
                                                                            == Some(selected.as_str())
                                                                        {
                                                                            "btn btn-sm btn-primary"
                                                                        } else {
                                                                            "btn btn-sm btn-outline"
                                                                        }
                                                                    }
                                                                    on:click=sub_click
                                                                >
                                                                    {label}
                                                                </button>
                                                            }
                                                        })
                                                        .collect_view()}
                                                </div>
                                            </div>
                                        }
                                            .into_any()
                                    } else {
                                        view! {
                                            <p class="text-sm opacity-70">
                                                {format!("{title} unlocks at level {unlock}.")}
                                            </p>
                                        }
                                            .into_any()
                                    },
                                )
                            }}
                        </div>
                    }
                }
            />
        </div>

        <h3 class="font-semibold text-sm mt-2">"Add a class"</h3>
        <Suspense fallback=move || {
            view! { <p>"Loading classes..."</p> }
        }>
            {move || Suspend::new(async move {
                match class_list.await {
                    Ok(rows) => {
                        let added: HashSet<String> = classes
                            .get()
                            .iter()
                            .map(|c| c.class_id.clone())
                            .collect();
                        let maxed = total() >= 20;
                        view! {
                            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                                {rows
                                    .into_iter()
                                    .filter(|class| !added.contains(&class.id))
                                    .map(|class| {
                                        let id = class.id.clone();
                                        let saves = class
                                            .saving_throws
                                            .iter()
                                            .map(|code| ability_label(code))
                                            .collect::<Vec<_>>()
                                            .join(", ");
                                        let prereqs = class
                                            .multiclass_ability_prerequisites
                                            .clone();
                                        let warning = move || unmet_prereq_label(
                                            &prereqs,
                                            !classes.get().is_empty(),
                                        );
                                        view! {
                                            <button
                                                type="button"
                                                class="btn btn-outline justify-start h-auto py-2 flex-col items-start w-full"
                                                disabled=maxed
                                                on:click=move |_| {
                                                    classes
                                                        .update(|list| {
                                                            list.push(ClassLevel {
                                                                class_id: id.clone(),
                                                                subclass_id: None,
                                                                level: 1,
                                                            });
                                                        });
                                                }
                                            >
                                                <span class="text-base">
                                                    {format!("{} ({})", class.name, class.source)}
                                                </span>
                                                <span class="text-xs font-normal opacity-70">
                                                    {format!("d{} · Saves: {}", class.hit_die, saves)}
                                                </span>
                                                {move || {
                                                    warning()
                                                        .map(|label| {
                                                            view! { <span class="text-warning text-xs">{label}</span> }
                                                        })
                                                }}
                                            </button>
                                        }
                                    })
                                    .collect_view()}
                            </div>
                            {maxed
                                .then(|| {
                                    view! {
                                        <p class="font-mono text-xs opacity-70">
                                            "20/20 — no more classes can be added."
                                        </p>
                                    }
                                })}
                        }
                            .into_any()
                    }
                    Err(err) => {
                        view! {
                            <p class="text-error">{format!("Failed to load classes: {err}")}</p>
                        }
                            .into_any()
                    }
                }
            })}
        </Suspense>
    }
}
