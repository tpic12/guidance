use crate::components::filters::{filter_button, filter_chip_group, filter_dialog, FilterOptions};
use crate::components::search_box::SearchBox;
use crate::components::species_detail_card::SpeciesDetailCard;
use crate::models::species::{Species, SpeciesQuery, SpeciesSort};
use leptos::prelude::*;

#[server]
pub async fn get_species(query: SpeciesQuery) -> Result<Vec<Species>, ServerFnError> {
    use sqlx::SqlitePool;

    let pool = expect_context::<SqlitePool>();
    crate::db::list_species(&pool, &query)
        .await
        .map_err(|err| ServerFnError::new(err.to_string()))
}

#[server]
pub async fn get_species_sources() -> Result<Vec<String>, ServerFnError> {
    use sqlx::SqlitePool;

    let pool = expect_context::<SqlitePool>();
    crate::db::list_species_sources(&pool)
        .await
        .map_err(|err| ServerFnError::new(err.to_string()))
}

#[server]
pub async fn get_species_by_id(id: String) -> Result<Option<Species>, ServerFnError> {
    use sqlx::SqlitePool;

    let pool = expect_context::<SqlitePool>();
    crate::db::get_species(&pool, &id).await.map_err(|err| ServerFnError::new(err.to_string()))
}

#[component]
pub fn SpeciesPage() -> impl IntoView {
    let search = RwSignal::new(String::new());
    let selected_sources = RwSignal::new(Vec::<String>::new());
    let sort = RwSignal::new(SpeciesSort::default());
    let selected_species = RwSignal::new(None::<Species>);

    let draft_sources = RwSignal::new(Vec::<String>::new());

    let filters_dialog = NodeRef::<leptos::html::Dialog>::new();

    let species = Resource::new(
        move || SpeciesQuery {
            search: search.get(),
            sources: selected_sources.get(),
            sort: sort.get(),
        },
        |query| async move { get_species(query).await },
    );

    let available_sources = Resource::new(|| (), |_| async move { get_species_sources().await });

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
            *s = if *s == SpeciesSort::NameAsc {
                SpeciesSort::NameDesc
            } else {
                SpeciesSort::NameAsc
            }
        })
    };
    let toggle_source_sort = move |_| {
        sort.update(|s| {
            *s = if *s == SpeciesSort::SourceAsc {
                SpeciesSort::SourceDesc
            } else {
                SpeciesSort::SourceAsc
            }
        })
    };

    let active_filter_count = move || selected_sources.get().len();

    view! {
        <div class="flex flex-col gap-4">
            <a href="/compendium" class="btn btn-ghost btn-sm w-fit gap-2">
                "← Back to Compendium"
            </a>
            <h1 class="text-3xl">"Species"</h1>

            <div class="flex flex-wrap gap-2 items-center">
                <SearchBox value=search placeholder="Search species..." />
                {filter_button(active_filter_count, open_filters)}
            </div>

            {filter_dialog(
                filters_dialog,
                "Filter Species",
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
                                        SpeciesSort::NameAsc,
                                        SpeciesSort::NameDesc,
                                    )}
                                </th>
                                <th>"Ability"</th>
                                <th>"Size"</th>
                                <th class="cursor-pointer select-none" on:click=toggle_source_sort>
                                    "Source"
                                    {move || sort_indicator(
                                        sort.get(),
                                        SpeciesSort::SourceAsc,
                                        SpeciesSort::SourceDesc,
                                    )}
                                </th>
                            </tr>
                        </thead>
                        <tbody>
                            <Suspense fallback=move || {
                                view! {
                                    <tr>
                                        <td colspan="4">"Loading species..."</td>
                                    </tr>
                                }
                            }>
                                {move || Suspend::new(async move {
                                    match species.await {
                                        Ok(rows) if rows.is_empty() => {
                                            view! {
                                                <tr>
                                                    <td colspan="4">"No species match your filters."</td>
                                                </tr>
                                            }
                                                .into_any()
                                        }
                                        Ok(rows) => {
                                            rows.into_iter()
                                                .map(|species| {
                                                    let row_species = species.clone();
                                                    view! {
                                                        <tr
                                                            class="row-hover cursor-pointer"
                                                            on:click=move |_| {
                                                                selected_species.set(Some(row_species.clone()))
                                                            }
                                                        >
                                                            <td>{species.name.clone()}</td>
                                                            <td>{dash_if_none(&species.ability)}</td>
                                                            <td>{dash_if_none(&species.size)}</td>
                                                            <td>{species.source.clone()}</td>
                                                        </tr>
                                                    }
                                                })
                                                .collect_view()
                                                .into_any()
                                        }
                                        Err(err) => {
                                            view! {
                                                <tr>
                                                    <td colspan="4">
                                                        {format!("Failed to load species: {err}")}
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
                        selected_species
                            .get()
                            .map(|species| view! { <SpeciesDetailCard species=species /> })
                    }}
                </div>
            </div>
        </div>
    }
}

fn sort_indicator(current: SpeciesSort, asc: SpeciesSort, desc: SpeciesSort) -> &'static str {
    if current == asc {
        " \u{25B2}"
    } else if current == desc {
        " \u{25BC}"
    } else {
        ""
    }
}

fn dash_if_none(value: &Option<String>) -> String {
    value.clone().unwrap_or_else(|| "\u{2014}".to_string())
}
