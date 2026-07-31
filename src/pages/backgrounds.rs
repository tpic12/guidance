use crate::components::background_detail_card::BackgroundDetailCard;
use crate::components::filters::{filter_button, filter_chip_group, filter_dialog, FilterOptions};
use crate::components::search_box::SearchBox;
use crate::models::background::{Background, BackgroundQuery, BackgroundSort};
use crate::models::skill::describe_skill_grants;
use leptos::prelude::*;

#[server]
pub async fn get_backgrounds(query: BackgroundQuery) -> Result<Vec<Background>, ServerFnError> {
    use sqlx::SqlitePool;

    let pool = expect_context::<SqlitePool>();
    crate::db::list_backgrounds(&pool, &query)
        .await
        .map_err(|err| ServerFnError::new(err.to_string()))
}

#[server]
pub async fn get_background_sources() -> Result<Vec<String>, ServerFnError> {
    use sqlx::SqlitePool;

    let pool = expect_context::<SqlitePool>();
    crate::db::list_background_sources(&pool)
        .await
        .map_err(|err| ServerFnError::new(err.to_string()))
}

#[server]
pub async fn get_background(id: String) -> Result<Option<Background>, ServerFnError> {
    use sqlx::SqlitePool;

    let pool = expect_context::<SqlitePool>();
    crate::db::get_background(&pool, &id).await.map_err(|err| ServerFnError::new(err.to_string()))
}

#[component]
pub fn BackgroundsPage() -> impl IntoView {
    let search = RwSignal::new(String::new());
    let selected_sources = RwSignal::new(Vec::<String>::new());
    let sort = RwSignal::new(BackgroundSort::default());
    let selected_background = RwSignal::new(None::<Background>);

    let draft_sources = RwSignal::new(Vec::<String>::new());

    let filters_dialog = NodeRef::<leptos::html::Dialog>::new();

    let backgrounds = Resource::new(
        move || BackgroundQuery {
            search: search.get(),
            sources: selected_sources.get(),
            sort: sort.get(),
        },
        |query| async move { get_backgrounds(query).await },
    );

    let available_sources = Resource::new(|| (), |_| async move { get_background_sources().await });

    let open_filters = move || {
        draft_sources.set(selected_sources.get());
        if let Some(dialog) = filters_dialog.get() {
            let _ = dialog.show_modal();
        }
    };

    let close_filters = move || {
        if let Some(dialog) = filters_dialog.get() {
            dialog.close();
        }
    };

    let save_filters = move || {
        selected_sources.set(draft_sources.get());
        if let Some(dialog) = filters_dialog.get() {
            dialog.close();
        }
    };

    let toggle_name_sort = move |_| {
        sort.update(|s| {
            *s = if *s == BackgroundSort::NameAsc {
                BackgroundSort::NameDesc
            } else {
                BackgroundSort::NameAsc
            }
        })
    };
    let toggle_source_sort = move |_| {
        sort.update(|s| {
            *s = if *s == BackgroundSort::SourceAsc {
                BackgroundSort::SourceDesc
            } else {
                BackgroundSort::SourceAsc
            }
        })
    };

    let active_filter_count = move || selected_sources.get().len();

    view! {
        <div class="flex flex-col gap-4">
            <a href="/compendium" class="btn btn-ghost btn-sm w-fit gap-2">
                "← Back to Compendium"
            </a>
            <h1 class="text-3xl">"Backgrounds"</h1>

            <div class="flex flex-wrap gap-2 items-center">
                <SearchBox value=search placeholder="Search backgrounds..." />
                {filter_button(active_filter_count, open_filters)}
            </div>

            {filter_dialog(
                filters_dialog,
                "Filter Backgrounds",
                close_filters,
                save_filters,
                vec![
                    filter_chip_group(
                            "Source",
                            FilterOptions::Async(available_sources),
                            |source: &String| source.clone(),
                            draft_sources,
                        )
                        .into_any(),
                ],
            )}

            <div class="flex flex-col lg:flex-row gap-4 items-start">
                <div class="overflow-auto w-full flex-1 max-h-[calc(100vh-16rem)] border border-base-300 rounded-box">
                    <table class="table">
                        <thead class="sticky top-0 z-10 bg-base-100">
                            <tr>
                                <th class="cursor-pointer select-none" on:click=toggle_name_sort>
                                    "Name"
                                    {move || sort_indicator(
                                        sort.get(),
                                        BackgroundSort::NameAsc,
                                        BackgroundSort::NameDesc,
                                    )}
                                </th>
                                <th>"Skill Proficiencies"</th>
                                <th class="cursor-pointer select-none" on:click=toggle_source_sort>
                                    "Source"
                                    {move || sort_indicator(
                                        sort.get(),
                                        BackgroundSort::SourceAsc,
                                        BackgroundSort::SourceDesc,
                                    )}
                                </th>
                            </tr>
                        </thead>
                        <tbody>
                            <Suspense fallback=move || {
                                view! {
                                    <tr>
                                        <td colspan="3">"Loading backgrounds..."</td>
                                    </tr>
                                }
                            }>
                                {move || Suspend::new(async move {
                                    match backgrounds.await {
                                        Ok(rows) if rows.is_empty() => {
                                            view! {
                                                <tr>
                                                    <td colspan="3">"No backgrounds match your filters."</td>
                                                </tr>
                                            }
                                                .into_any()
                                        }
                                        Ok(rows) => {
                                            rows.into_iter()
                                                .map(|background| {
                                                    let row_background = background.clone();
                                                    view! {
                                                        <tr
                                                            class="row-hover cursor-pointer"
                                                            on:click=move |_| {
                                                                selected_background.set(Some(row_background.clone()))
                                                            }
                                                        >
                                                            <td>{background.name.clone()}</td>
                                                            <td>
                                                                {if background.skills.is_empty() {
                                                                    "\u{2014}".to_string()
                                                                } else {
                                                                    describe_skill_grants(&background.skills)
                                                                }}
                                                            </td>
                                                            <td>{background.source.clone()}</td>
                                                        </tr>
                                                    }
                                                })
                                                .collect_view()
                                                .into_any()
                                        }
                                        Err(err) => {
                                            view! {
                                                <tr>
                                                    <td colspan="3">
                                                        {format!("Failed to load backgrounds: {err}")}
                                                    </td>
                                                </tr>
                                            }
                                                .into_any()
                                        }
                                    }
                                })}
                            </Suspense>
                        </tbody>
                    </table>
                </div>

                <div class="w-full lg:w-96 max-h-[calc(100vh-16rem)] overflow-y-auto">
                    {move || {
                        selected_background
                            .get()
                            .map(|background| view! { <BackgroundDetailCard background=background /> })
                    }}
                </div>
            </div>
        </div>
    }
}

fn sort_indicator(
    current: BackgroundSort,
    asc: BackgroundSort,
    desc: BackgroundSort,
) -> &'static str {
    if current == asc {
        " \u{25B2}"
    } else if current == desc {
        " \u{25BC}"
    } else {
        ""
    }
}

