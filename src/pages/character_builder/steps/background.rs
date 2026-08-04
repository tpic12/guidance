use crate::components::background_detail_card::BackgroundDetailCard;
use crate::models::background::Background;
use crate::models::skill::describe_skill_grants;
use leptos::prelude::*;

use super::detail_panel_placeholder;

pub fn background_step(
    background_search: RwSignal<String>,
    background_list: Resource<Result<Vec<Background>, ServerFnError>>,
    background_id: RwSignal<Option<String>>,
) -> impl IntoView {
    let focused_background = RwSignal::new(None::<Background>);
    // Same fallback reasoning as species_step's selected_species — the
    // detail panel reverts to the committed pick (or the placeholder, if
    // none) once the mouse leaves the list without a new hover/click.
    let selected_background = move || -> Option<Background> {
        let id = background_id.get()?;
        background_list.get()?.ok()?.into_iter().find(|b| b.id == id)
    };

    view! {
        <h2 class="card-title">"Background"</h2>
        <div class="flex flex-col lg:flex-row gap-4 items-start">
            <div class="flex-1 w-full flex flex-col gap-2">
                <input
                    type="text"
                    class="input input-bordered w-full max-w-sm"
                    placeholder="Search backgrounds..."
                    prop:value=move || background_search.get()
                    on:input=move |ev| background_search.set(event_target_value(&ev))
                />
                <ul
                    class="menu bg-base-200 rounded-box max-h-96 overflow-y-auto flex-nowrap w-full"
                    on:mouseleave=move |_| focused_background.set(selected_background())
                >
                    {move || match background_list.get() {
                        None => {
                            view! { <li class="p-2 text-sm opacity-60">"Loading..."</li> }
                                .into_any()
                        }
                        Some(Err(err)) => {
                            view! {
                                <li class="p-2 text-sm text-error">
                                    {format!("Failed to load: {err}")}
                                </li>
                            }
                                .into_any()
                        }
                        Some(Ok(rows)) if rows.is_empty() => {
                            view! { <li class="p-2 text-sm opacity-60">"No matches."</li> }
                                .into_any()
                        }
                        Some(Ok(rows)) => {
                            rows.into_iter()
                                .map(|background| {
                                    let id = background.id.clone();
                                    let is_selected = move || {
                                        background_id.get().as_deref() == Some(id.as_str())
                                    };
                                    let click_id = background.id.clone();
                                    let hover_background = background.clone();
                                    let click_background = background.clone();
                                    let title = format!(
                                        "{} ({})",
                                        background.name,
                                        background.source,
                                    );
                                    let subtitle = if background.skills.is_empty() {
                                        "\u{2014}".to_string()
                                    } else {
                                        describe_skill_grants(&background.skills)
                                    };
                                    view! {
                                        <li>
                                            <button
                                                type="button"
                                                class=move || {
                                                    if is_selected() {
                                                        "flex flex-col items-start gap-0 bg-primary/15 border border-primary/50"
                                                    } else {
                                                        "flex flex-col items-start gap-0 border border-transparent"
                                                    }
                                                }
                                                on:mouseenter=move |_| {
                                                    focused_background.set(Some(hover_background.clone()))
                                                }
                                                on:click=move |_| {
                                                    focused_background.set(Some(click_background.clone()));
                                                    background_id.set(Some(click_id.clone()));
                                                }
                                            >
                                                <span>{title}</span>
                                                <span class="text-xs opacity-70">{subtitle}</span>
                                            </button>
                                        </li>
                                    }
                                })
                                .collect_view()
                                .into_any()
                        }
                    }}
                </ul>
            </div>
            <div class="flex-1 w-full lg:sticky lg:top-4">
                {move || match focused_background.get() {
                    Some(background) => {
                        view! { <BackgroundDetailCard background=background /> }.into_any()
                    }
                    None => {
                        detail_panel_placeholder("Hover or select a background to see its details.")
                            .into_any()
                    }
                }}
            </div>
        </div>
    }
}
