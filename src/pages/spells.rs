use crate::components::filters::{filter_button, filter_chip_group, filter_dialog, FilterOptions};
use crate::components::search_box::SearchBox;
use crate::components::spell_detail_card::SpellDetailCard;
use crate::models::spell::{School, Spell, SpellQuery, SpellSort};
use leptos::prelude::*;

#[server]
pub async fn get_spells(query: SpellQuery) -> Result<Vec<Spell>, ServerFnError> {
    use sqlx::SqlitePool;

    let pool = expect_context::<SqlitePool>();
    crate::db::list_spells(&pool, &query)
        .await
        .map_err(|err| ServerFnError::new(err.to_string()))
}

#[server]
pub async fn get_spell_sources() -> Result<Vec<String>, ServerFnError> {
    use sqlx::SqlitePool;

    let pool = expect_context::<SqlitePool>();
    crate::db::list_spell_sources(&pool)
        .await
        .map_err(|err| ServerFnError::new(err.to_string()))
}

#[server]
pub async fn get_spell_classes() -> Result<Vec<String>, ServerFnError> {
    use sqlx::SqlitePool;

    let pool = expect_context::<SqlitePool>();
    crate::db::list_spell_classes(&pool)
        .await
        .map_err(|err| ServerFnError::new(err.to_string()))
}

#[server]
pub async fn get_class_spell_options(
    class_id: String,
    subclass_id: Option<String>,
    level: u8,
    max_spell_level: u8,
) -> Result<Vec<Spell>, ServerFnError> {
    use sqlx::SqlitePool;

    let pool = expect_context::<SqlitePool>();
    crate::db::list_spells_for_class(&pool, &class_id, subclass_id.as_deref(), level, max_spell_level)
        .await
        .map_err(|err| ServerFnError::new(err.to_string()))
}

#[server]
pub async fn get_subclass_granted_spells(
    subclass_id: String,
    level: u8,
) -> Result<Vec<Spell>, ServerFnError> {
    use sqlx::SqlitePool;

    let pool = expect_context::<SqlitePool>();
    crate::db::get_subclass_granted_spells(&pool, &subclass_id, level)
        .await
        .map_err(|err| ServerFnError::new(err.to_string()))
}

#[component]
pub fn SpellsPage() -> impl IntoView {
    let search = RwSignal::new(String::new());
    let selected_schools = RwSignal::new(Vec::<School>::new());
    let selected_levels = RwSignal::new(Vec::<u8>::new());
    let selected_sources = RwSignal::new(Vec::<String>::new());
    let selected_classes = RwSignal::new(Vec::<String>::new());
    let sort = RwSignal::new(SpellSort::default());
    let selected_spell = RwSignal::new(None::<Spell>);

    let draft_schools = RwSignal::new(Vec::<School>::new());
    let draft_levels = RwSignal::new(Vec::<u8>::new());
    let draft_sources = RwSignal::new(Vec::<String>::new());
    let draft_classes = RwSignal::new(Vec::<String>::new());

    let filters_dialog = NodeRef::<leptos::html::Dialog>::new();

    let spells = Resource::new(
        move || SpellQuery {
            search: search.get(),
            schools: selected_schools.get(),
            levels: selected_levels.get(),
            sources: selected_sources.get(),
            classes: selected_classes.get(),
            sort: sort.get(),
        },
        |query| async move { get_spells(query).await },
    );

    let available_sources = Resource::new(|| (), |_| async move { get_spell_sources().await });
    let available_classes = Resource::new(|| (), |_| async move { get_spell_classes().await });

    let open_filters = move || {
        draft_schools.set(selected_schools.get());
        draft_levels.set(selected_levels.get());
        draft_sources.set(selected_sources.get());
        draft_classes.set(selected_classes.get());
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
        selected_schools.set(draft_schools.get());
        selected_levels.set(draft_levels.get());
        selected_sources.set(draft_sources.get());
        selected_classes.set(draft_classes.get());
        if let Some(dialog) = filters_dialog.get() {
            dialog.close();
        }
    };

    let toggle_name_sort = move |_| {
        sort.update(|s| *s = if *s == SpellSort::NameAsc { SpellSort::NameDesc } else { SpellSort::NameAsc })
    };
    let toggle_level_sort = move |_| {
        sort.update(|s| *s = if *s == SpellSort::LevelAsc { SpellSort::LevelDesc } else { SpellSort::LevelAsc })
    };
    let toggle_school_sort = move |_| {
        sort.update(|s| *s = if *s == SpellSort::SchoolAsc { SpellSort::SchoolDesc } else { SpellSort::SchoolAsc })
    };

    let active_filter_count = move || {
        selected_schools.get().len()
            + selected_levels.get().len()
            + selected_sources.get().len()
            + selected_classes.get().len()
    };

    let all_levels = move || (0u8..=9).collect::<Vec<u8>>();

    view! {
        <div class="flex flex-col gap-4">
            <a href="/compendium" class="btn btn-ghost btn-sm w-fit gap-2">
                "← Back to Compendium"
            </a>
            <h1 class="text-3xl">"Spells"</h1>

            <div class="flex flex-wrap gap-2 items-center">
                <SearchBox value=search placeholder="Search spells..." />
                {filter_button(active_filter_count, open_filters)}
            </div>

            {filter_dialog(
                filters_dialog,
                "Filter Spells",
                close_filters,
                save_filters,
                vec![
                    filter_chip_group(
                            "Schools",
                            FilterOptions::Static(School::ALL.to_vec()),
                            |school: &School| school.label().to_string(),
                            draft_schools,
                        )
                        .into_any(),
                    filter_chip_group(
                            "Levels",
                            FilterOptions::Static(all_levels()),
                            |level: &u8| level_badge(*level),
                            draft_levels,
                        )
                        .into_any(),
                    filter_chip_group(
                            "Source",
                            FilterOptions::Async(available_sources),
                            |source: &String| source.clone(),
                            draft_sources,
                        )
                        .into_any(),
                    filter_chip_group(
                            "Class",
                            FilterOptions::Async(available_classes),
                            |class_name: &String| class_name.clone(),
                            draft_classes,
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
                                        SpellSort::NameAsc,
                                        SpellSort::NameDesc,
                                    )}
                                </th>
                                <th class="cursor-pointer select-none" on:click=toggle_level_sort>
                                    "Level"
                                    {move || sort_indicator(
                                        sort.get(),
                                        SpellSort::LevelAsc,
                                        SpellSort::LevelDesc,
                                    )}
                                </th>
                                <th class="cursor-pointer select-none" on:click=toggle_school_sort>
                                    "School"
                                    {move || sort_indicator(
                                        sort.get(),
                                        SpellSort::SchoolAsc,
                                        SpellSort::SchoolDesc,
                                    )}
                                </th>
                                <th>"Source"</th>
                            </tr>
                        </thead>
                        <tbody>
                            <Suspense fallback=move || {
                                view! {
                                    <tr>
                                        <td colspan="4">"Loading spells..."</td>
                                    </tr>
                                }
                            }>
                                {move || Suspend::new(async move {
                                    match spells.await {
                                        Ok(rows) if rows.is_empty() => {
                                            view! {
                                                <tr>
                                                    <td colspan="4">"No spells match your filters."</td>
                                                </tr>
                                            }
                                                .into_any()
                                        }
                                        Ok(rows) => {
                                            rows.into_iter()
                                                .map(|spell| {
                                                    let row_spell = spell.clone();
                                                    view! {
                                                        <tr
                                                            class="row-hover cursor-pointer"
                                                            on:click=move |_| {
                                                                selected_spell.set(Some(row_spell.clone()))
                                                            }
                                                        >
                                                            <td>{spell.name.clone()}</td>
                                                            <td>{level_badge(spell.level)}</td>
                                                            <td>{spell.school.label()}</td>
                                                            <td>{spell.source.clone()}</td>
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
                                                        {format!("Failed to load spells: {err}")}
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
                        selected_spell.get().map(|spell| view! { <SpellDetailCard spell=spell /> })
                    }}
                </div>
            </div>
        </div>
    }
}

fn sort_indicator(current: SpellSort, asc: SpellSort, desc: SpellSort) -> &'static str {
    if current == asc {
        " \u{25B2}"
    } else if current == desc {
        " \u{25BC}"
    } else {
        ""
    }
}

fn level_badge(level: u8) -> String {
    if level == 0 { "Cantrip".to_string() } else { level.to_string() }
}
