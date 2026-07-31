use crate::components::filters::{filter_button, filter_chip_group, filter_dialog, FilterOptions};
use crate::components::optional_feature_detail_card::{type_badges, OptionalFeatureDetailCard};
use crate::components::search_box::SearchBox;
use crate::models::optional_feature::{
    prerequisite_label, FeatureType, OptionalFeature, OptionalFeatureQuery, OptionalFeatureSort,
};
use leptos::prelude::*;

#[server]
pub async fn get_optional_features(
    query: OptionalFeatureQuery,
) -> Result<Vec<OptionalFeature>, ServerFnError> {
    use sqlx::SqlitePool;

    let pool = expect_context::<SqlitePool>();
    crate::db::list_optional_features(&pool, &query)
        .await
        .map_err(|err| ServerFnError::new(err.to_string()))
}

#[server]
pub async fn get_optional_feature_sources() -> Result<Vec<String>, ServerFnError> {
    use sqlx::SqlitePool;

    let pool = expect_context::<SqlitePool>();
    crate::db::list_optional_feature_sources(&pool)
        .await
        .map_err(|err| ServerFnError::new(err.to_string()))
}

#[server]
pub async fn get_optional_feature_class_requirements() -> Result<Vec<String>, ServerFnError> {
    use sqlx::SqlitePool;

    let pool = expect_context::<SqlitePool>();
    crate::db::list_optional_feature_class_requirements(&pool)
        .await
        .map_err(|err| ServerFnError::new(err.to_string()))
}

#[server]
pub async fn get_optional_feature_pacts() -> Result<Vec<String>, ServerFnError> {
    use sqlx::SqlitePool;

    let pool = expect_context::<SqlitePool>();
    crate::db::list_optional_feature_pacts(&pool)
        .await
        .map_err(|err| ServerFnError::new(err.to_string()))
}

#[component]
pub fn OptionalFeaturesPage() -> impl IntoView {
    let search = RwSignal::new(String::new());
    let selected_type_groups = RwSignal::new(Vec::<FeatureTypeGroup>::new());
    let selected_sources = RwSignal::new(Vec::<String>::new());
    let selected_class_requirements = RwSignal::new(Vec::<String>::new());
    let selected_pacts = RwSignal::new(Vec::<String>::new());
    let sort = RwSignal::new(OptionalFeatureSort::default());
    let selected_feature = RwSignal::new(None::<OptionalFeature>);

    let draft_type_groups = RwSignal::new(Vec::<FeatureTypeGroup>::new());
    let draft_sources = RwSignal::new(Vec::<String>::new());
    let draft_class_requirements = RwSignal::new(Vec::<String>::new());
    let draft_pacts = RwSignal::new(Vec::<String>::new());

    let filters_dialog = NodeRef::<leptos::html::Dialog>::new();

    let features = Resource::new(
        move || OptionalFeatureQuery {
            search: search.get(),
            types: selected_type_groups.get().iter().flat_map(|group| group.members.iter().copied()).collect(),
            sources: selected_sources.get(),
            class_requirements: selected_class_requirements.get(),
            pacts: selected_pacts.get(),
            sort: sort.get(),
        },
        |query| async move { get_optional_features(query).await },
    );

    let available_sources = Resource::new(|| (), |_| async move { get_optional_feature_sources().await });
    let available_class_requirements =
        Resource::new(|| (), |_| async move { get_optional_feature_class_requirements().await });
    let available_pacts = Resource::new(|| (), |_| async move { get_optional_feature_pacts().await });

    let open_filters = move || {
        draft_type_groups.set(selected_type_groups.get());
        draft_sources.set(selected_sources.get());
        draft_class_requirements.set(selected_class_requirements.get());
        draft_pacts.set(selected_pacts.get());
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
        selected_type_groups.set(draft_type_groups.get());
        selected_sources.set(draft_sources.get());
        selected_class_requirements.set(draft_class_requirements.get());
        selected_pacts.set(draft_pacts.get());
        if let Some(dialog) = filters_dialog.get() {
            dialog.close();
        }
    };

    let toggle_name_sort = move |_| {
        sort.update(|s| {
            *s = if *s == OptionalFeatureSort::NameAsc {
                OptionalFeatureSort::NameDesc
            } else {
                OptionalFeatureSort::NameAsc
            }
        })
    };
    let toggle_source_sort = move |_| {
        sort.update(|s| {
            *s = if *s == OptionalFeatureSort::SourceAsc {
                OptionalFeatureSort::SourceDesc
            } else {
                OptionalFeatureSort::SourceAsc
            }
        })
    };

    let active_filter_count = move || {
        selected_type_groups.get().len()
            + selected_sources.get().len()
            + selected_class_requirements.get().len()
            + selected_pacts.get().len()
    };

    view! {
        <div class="flex flex-col gap-4">
            <a href="/compendium" class="btn btn-ghost btn-sm w-fit gap-2">
                "← Back to Compendium"
            </a>
            <h1 class="text-3xl">"Optional Features"</h1>

            <div class="flex flex-wrap gap-2 items-center">
                <SearchBox value=search placeholder="Search optional features..." />
                {filter_button(active_filter_count, open_filters)}
            </div>

            {filter_dialog(
                filters_dialog,
                "Filter Optional Features",
                close_filters,
                save_filters,
                vec![
                    filter_chip_group(
                            "Type",
                            FilterOptions::Static(FEATURE_TYPE_GROUPS.to_vec()),
                            |group: &FeatureTypeGroup| group.label.to_string(),
                            draft_type_groups,
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
                            FilterOptions::Async(available_class_requirements),
                            |value: &String| value.clone(),
                            draft_class_requirements,
                        )
                        .into_any(),
                    filter_chip_group(
                            "Pact",
                            FilterOptions::Async(available_pacts),
                            |value: &String| value.clone(),
                            draft_pacts,
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
                                        OptionalFeatureSort::NameAsc,
                                        OptionalFeatureSort::NameDesc,
                                    )}
                                </th>
                                <th>"Type"</th>
                                <th>"Prerequisite"</th>
                                <th class="cursor-pointer select-none" on:click=toggle_source_sort>
                                    "Source"
                                    {move || sort_indicator(
                                        sort.get(),
                                        OptionalFeatureSort::SourceAsc,
                                        OptionalFeatureSort::SourceDesc,
                                    )}
                                </th>
                            </tr>
                        </thead>
                        <tbody>
                            <Suspense fallback=move || {
                                view! {
                                    <tr>
                                        <td colspan="4">"Loading optional features..."</td>
                                    </tr>
                                }
                            }>
                                {move || Suspend::new(async move {
                                    match features.await {
                                        Ok(rows) if rows.is_empty() => {
                                            view! {
                                                <tr>
                                                    <td colspan="4">
                                                        "No optional features match your filters."
                                                    </td>
                                                </tr>
                                            }
                                                .into_any()
                                        }
                                        Ok(rows) => {
                                            rows.into_iter()
                                                .map(|feature| {
                                                    let row_feature = feature.clone();
                                                    view! {
                                                        <tr
                                                            class="row-hover cursor-pointer"
                                                            on:click=move |_| {
                                                                selected_feature.set(Some(row_feature.clone()))
                                                            }
                                                        >
                                                            <td>{feature.name.clone()}</td>
                                                            <td>{type_badges(&feature.feature_types)}</td>
                                                            <td>{prerequisite_cell(&feature)}</td>
                                                            <td>{feature.source.clone()}</td>
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
                                                        {format!("Failed to load optional features: {err}")}
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
                        selected_feature.get().map(|feature| view! { <OptionalFeatureDetailCard feature=feature /> })
                    }}
                </div>
            </div>
        </div>
    }
}

/// A compendium-facing Type filter option — collapses the 4 class-scoped
/// Fighting Style `FeatureType` variants into one "Fighting Style" chip,
/// since which class grants it doesn't matter for browsing (only for
/// character creation, which filters on the raw `FeatureType`s directly).
#[derive(Clone, PartialEq)]
struct FeatureTypeGroup {
    label: &'static str,
    members: &'static [FeatureType],
}

const FEATURE_TYPE_GROUPS: &[FeatureTypeGroup] = &[
    FeatureTypeGroup { label: "Eldritch Invocation", members: &[FeatureType::EldritchInvocation] },
    FeatureTypeGroup { label: "Pact Boon", members: &[FeatureType::PactBoon] },
    FeatureTypeGroup { label: "Metamagic", members: &[FeatureType::Metamagic] },
    FeatureTypeGroup { label: "Infusion", members: &[FeatureType::ArtificerInfusion] },
    FeatureTypeGroup { label: "Maneuver", members: &[FeatureType::Maneuver] },
    FeatureTypeGroup { label: "Arcane Shot", members: &[FeatureType::ArcaneShot] },
    FeatureTypeGroup { label: "Elemental Discipline", members: &[FeatureType::ElementalDiscipline] },
    FeatureTypeGroup { label: "Rune", members: &[FeatureType::Rune] },
    FeatureTypeGroup {
        label: "Fighting Style",
        members: &[
            FeatureType::FightingStyleFighter,
            FeatureType::FightingStyleRanger,
            FeatureType::FightingStylePaladin,
            FeatureType::FightingStyleBard,
        ],
    },
];

fn prerequisite_cell(feature: &OptionalFeature) -> String {
    prerequisite_label(&feature.prerequisites).unwrap_or_else(|| "\u{2014}".to_string())
}

fn sort_indicator(
    current: OptionalFeatureSort,
    asc: OptionalFeatureSort,
    desc: OptionalFeatureSort,
) -> &'static str {
    if current == asc {
        " \u{25B2}"
    } else if current == desc {
        " \u{25BC}"
    } else {
        ""
    }
}

