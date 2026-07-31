use crate::components::entry_view::entry_view;
use crate::models::class::{ability_label, ClassDetail, ClassFeature};
use crate::models::skill::describe_skill_grants;
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

#[server]
pub async fn get_class_detail(id: String) -> Result<Option<ClassDetail>, ServerFnError> {
    use sqlx::SqlitePool;

    let pool = expect_context::<SqlitePool>();
    crate::db::get_class_detail(&pool, &id)
        .await
        .map_err(|err| ServerFnError::new(err.to_string()))
}

/// Per-subclass accent styles (toggle chip, feature border, name badge).
/// Assigned cyclically by subclass index; full literal class names so the
/// Tailwind scanner picks them up.
const SUBCLASS_STYLES: [(&str, &str, &str); 7] = [
    ("btn-primary", "border-primary", "badge-primary"),
    ("btn-secondary", "border-secondary", "badge-secondary"),
    ("btn-accent", "border-accent", "badge-accent"),
    ("btn-info", "border-info", "badge-info"),
    ("btn-success", "border-success", "badge-success"),
    ("btn-warning", "border-warning", "badge-warning"),
    ("btn-error", "border-error", "badge-error"),
];

fn subclass_style(index: usize) -> (&'static str, &'static str, &'static str) {
    SUBCLASS_STYLES[index % SUBCLASS_STYLES.len()]
}

#[component]
pub fn ClassDetailPage() -> impl IntoView {
    let params = use_params_map();
    let detail = Resource::new(
        move || params.read().get("id").unwrap_or_default(),
        |id| async move { get_class_detail(id).await },
    );

    view! {
        <div class="flex flex-col gap-4">
            <a href="/compendium/classes" class="btn btn-ghost btn-sm w-fit gap-2">
                "← Back to Classes"
            </a>
            <Suspense fallback=move || {
                view! { <p>"Loading class..."</p> }
            }>
                {move || Suspend::new(async move {
                    match detail.await {
                        Ok(Some(detail)) => view! { <ClassDetailView detail=detail /> }.into_any(),
                        Ok(None) => {
                            view! { <p class="opacity-70">"Class not found."</p> }.into_any()
                        }
                        Err(err) => {
                            view! {
                                <p class="text-error">{format!("Failed to load class: {err}")}</p>
                            }
                                .into_any()
                        }
                    }
                })}
            </Suspense>
        </div>
    }
}

#[component]
fn ClassDetailView(detail: ClassDetail) -> impl IntoView {
    let class = detail.class.clone();
    let selected = RwSignal::new(Vec::<String>::new());

    let saves = class
        .saving_throws
        .iter()
        .map(|code| ability_label(code))
        .collect::<Vec<_>>()
        .join(", ");
    let spellcasting = class.spellcasting_ability.as_deref().map(|ability| {
        match class.caster_progression.as_deref() {
            Some("full") => format!("{} (full caster)", ability_label(ability)),
            Some("half") | Some("artificer") => {
                format!("{} (half caster)", ability_label(ability))
            }
            Some("third") => format!("{} (third caster)", ability_label(ability)),
            Some("pact") => format!("{} (pact magic)", ability_label(ability)),
            _ => ability_label(ability).to_string(),
        }
    });

    // Progression table header. Titled column groups (e.g. "Spell Slots per
    // Spell Level") get a spanning cell in the first row with their column
    // labels in a second row; everything else spans both rows.
    let has_titled_groups = class.table_groups.iter().any(|group| group.title.is_some());
    let base_rowspan = if has_titled_groups { "2" } else { "1" };
    let mut header_row1: Vec<AnyView> = ["Level", "Prof. Bonus", "Features"]
        .into_iter()
        .map(|label| view! {
            <th rowspan=base_rowspan class="align-bottom">
                {label}
            </th>
        }.into_any())
        .collect();
    let mut header_row2: Vec<AnyView> = Vec::new();
    for group in &class.table_groups {
        match &group.title {
            Some(title) => {
                header_row1.push(
                    view! {
                        <th
                            colspan=group.col_labels.len().to_string()
                            class="text-center border-b border-base-300"
                        >
                            {title.clone()}
                        </th>
                    }
                    .into_any(),
                );
                for label in &group.col_labels {
                    header_row2.push(
                        view! { <th class="text-center">{label.clone()}</th> }.into_any(),
                    );
                }
            }
            None => {
                for label in &group.col_labels {
                    header_row1.push(
                        view! {
                            <th rowspan=base_rowspan class="align-bottom text-center">
                                {label.clone()}
                            </th>
                        }
                        .into_any(),
                    );
                }
            }
        }
    }

    let body_rows: Vec<AnyView> = (1u8..=20)
        .map(|level| {
            let feature_names: Vec<String> = detail
                .features
                .iter()
                .filter(|feature| feature.level == level)
                .map(|feature| feature.name.clone())
                .collect();
            let features_cell = if feature_names.is_empty() {
                "\u{2014}".to_string()
            } else {
                feature_names.join(", ")
            };

            let mut cells: Vec<AnyView> = vec![
                view! { <th class="whitespace-nowrap">{ordinal(level)}</th> }.into_any(),
                view! {
                    <td class="text-center">{format!("+{}", 2 + (level - 1) / 4)}</td>
                }
                .into_any(),
                view! { <td>{features_cell}</td> }.into_any(),
            ];
            for group in &class.table_groups {
                let row = group.rows.get(level as usize - 1);
                for col in 0..group.col_labels.len() {
                    let text = row
                        .and_then(|cells| cells.get(col))
                        .cloned()
                        .unwrap_or_else(|| "\u{2014}".to_string());
                    cells.push(
                        view! { <td class="text-center whitespace-nowrap">{text}</td> }
                            .into_any(),
                    );
                }
            }
            view! { <tr>{cells}</tr> }.into_any()
        })
        .collect();

    let class_source = class.source.clone();
    let subclass_chips: Vec<AnyView> = detail
        .subclasses
        .iter()
        .enumerate()
        .map(|(index, subclass)| {
            let id = subclass.subclass.id.clone();
            let toggle_id = id.clone();
            let (chip_color, _, _) = subclass_style(index);
            let chip_class = move || {
                if selected.get().contains(&id) {
                    format!("btn btn-sm {chip_color}")
                } else {
                    "btn btn-sm btn-outline".to_string()
                }
            };
            let source_badge = (subclass.subclass.source != class_source).then(|| {
                view! {
                    <span class="badge badge-xs badge-outline font-normal">
                        {subclass.subclass.source.clone()}
                    </span>
                }
            });
            view! {
                <button
                    type="button"
                    class=chip_class
                    on:click=move |_| {
                        selected
                            .update(|ids| match ids.iter().position(|x| *x == toggle_id) {
                                Some(pos) => {
                                    ids.remove(pos);
                                }
                                None => ids.push(toggle_id.clone()),
                            });
                    }
                >
                    {subclass.subclass.short_name.clone()}
                    {source_badge}
                </button>
            }
            .into_any()
        })
        .collect();

    let detail = StoredValue::new(detail);
    let features_by_level = move || {
        let selected_ids = selected.get();
        detail.with_value(|detail| {
            let class_source = detail.class.source.clone();
            (1u8..=20)
                .filter_map(|level| {
                    let class_features: Vec<AnyView> = detail
                        .features
                        .iter()
                        .filter(|feature| feature.level == level)
                        .map(|feature| feature_view(feature, &class_source))
                        .collect();

                    let subclass_features: Vec<AnyView> = detail
                        .subclasses
                        .iter()
                        .enumerate()
                        .filter(|(_, s)| selected_ids.contains(&s.subclass.id))
                        .filter_map(|(index, s)| {
                            let features: Vec<&ClassFeature> = s
                                .features
                                .iter()
                                .filter(|feature| feature.level == level)
                                .collect();
                            if features.is_empty() {
                                return None;
                            }
                            let (_, border_color, badge_color) = subclass_style(index);
                            let short_name = s.subclass.short_name.clone();
                            Some(
                                view! {
                                    <div class=format!(
                                        "border-l-4 {border_color} pl-4 space-y-4",
                                    )>
                                        {features
                                            .into_iter()
                                            .map(|feature| {
                                                subclass_feature_view(feature, &short_name, badge_color)
                                            })
                                            .collect_view()}
                                    </div>
                                }
                                .into_any(),
                            )
                        })
                        .collect();

                    if class_features.is_empty() && subclass_features.is_empty() {
                        return None;
                    }
                    Some(
                        view! {
                            <div class="space-y-4">
                                <div class="divider font-semibold">{format!("Level {level}")}</div>
                                {class_features}
                                {subclass_features}
                            </div>
                        }
                        .into_any(),
                    )
                })
                .collect_view()
        })
    };

    view! {
        <div class="flex flex-col gap-6 max-w-5xl">
            <header>
                <h1 class="text-3xl flex items-center gap-3">
                    {class.name.clone()}
                    <span class="badge badge-outline font-normal">{class.source.clone()}</span>
                </h1>
                <ul class="flex flex-wrap gap-x-6 gap-y-1 text-sm mt-2 opacity-90">
                    <li>
                        <span class="font-semibold">"Hit Die: "</span>
                        {format!("d{}", class.hit_die)}
                    </li>
                    <li>
                        <span class="font-semibold">"Saving Throws: "</span>
                        {saves}
                    </li>
                    {spellcasting
                        .map(|label| {
                            view! {
                                <li>
                                    <span class="font-semibold">"Spellcasting: "</span>
                                    {label}
                                </li>
                            }
                        })}
                </ul>
            </header>

            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div class="card bg-base-100 border border-base-300">
                    <div class="card-body py-4">
                        <h2 class="card-title text-base">"Proficiencies"</h2>
                        <ul class="text-sm space-y-1">
                            <li>
                                <span class="font-semibold">"Armor: "</span>
                                {list_or_none(&class.proficiencies.armor)}
                            </li>
                            <li>
                                <span class="font-semibold">"Weapons: "</span>
                                {list_or_none(&class.proficiencies.weapons)}
                            </li>
                            <li>
                                <span class="font-semibold">"Tools: "</span>
                                {list_or_none(&class.proficiencies.tools)}
                            </li>
                            <li>
                                <span class="font-semibold">"Skills: "</span>
                                {describe_skill_grants(&class.proficiencies.skills)}
                            </li>
                        </ul>
                    </div>
                </div>
                <div class="card bg-base-100 border border-base-300">
                    <div class="card-body py-4">
                        <h2 class="card-title text-base">"Starting Equipment"</h2>
                        <ul class="text-sm list-disc pl-5 space-y-1">
                            {class
                                .starting_equipment
                                .iter()
                                .map(|line| view! { <li>{line.clone()}</li> })
                                .collect_view()}
                        </ul>
                    </div>
                </div>
            </div>

            <section>
                <h2 class="text-2xl mb-2">"Level Progression"</h2>
                <div class="overflow-x-auto border border-base-300 rounded-box">
                    <table class="table table-sm table-zebra">
                        <thead>
                            <tr>{header_row1}</tr>
                            {has_titled_groups.then(|| view! { <tr>{header_row2}</tr> })}
                        </thead>
                        <tbody>{body_rows}</tbody>
                    </table>
                </div>
            </section>

            <section>
                <h2 class="text-2xl mb-1">{class.subclass_title.clone()}</h2>
                <p class="text-sm opacity-70 mb-3">
                    "Toggle one or more subclasses to weave their features into the level list below."
                </p>
                <div class="flex flex-wrap gap-2">{subclass_chips}</div>
            </section>

            <section>
                <h2 class="text-2xl">"Features"</h2>
                {features_by_level}
            </section>
        </div>
    }
}

fn feature_view(feature: &ClassFeature, class_source: &str) -> AnyView {
    let source_badge = (feature.source != class_source).then(|| {
        view! { <span class="badge badge-outline badge-sm font-normal">{feature.source.clone()}</span> }
    });
    view! {
        <section>
            <h3 class="text-lg font-semibold flex items-center gap-2">
                {feature.name.clone()} {source_badge}
            </h3>
            <div class="space-y-2 text-sm mt-1">
                {feature.entries.iter().map(entry_view).collect_view()}
            </div>
        </section>
    }
    .into_any()
}

fn subclass_feature_view(feature: &ClassFeature, subclass_name: &str, badge_color: &str) -> AnyView {
    view! {
        <section>
            <h3 class="text-lg font-semibold flex items-center gap-2">
                {feature.name.clone()}
                <span class=format!(
                    "badge badge-sm {badge_color}",
                )>{subclass_name.to_string()}</span>
            </h3>
            <div class="space-y-2 text-sm mt-1">
                {feature.entries.iter().map(entry_view).collect_view()}
            </div>
        </section>
    }
    .into_any()
}

fn list_or_none(items: &[String]) -> String {
    if items.is_empty() { "None".to_string() } else { items.join(", ") }
}

fn ordinal(n: u8) -> String {
    let suffix = match n % 100 {
        11..=13 => "th",
        _ => match n % 10 {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        },
    };
    format!("{n}{suffix}")
}
