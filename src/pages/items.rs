use crate::components::entry_view::entry_view;
use crate::components::filters::{filter_button, filter_chip_group, filter_dialog, FilterOptions};
use crate::components::search_box::SearchBox;
use crate::models::item::{
    AttunementRequirement, DamageType, Item, ItemKind, ItemMagicBonuses, ItemPropertyOption, ItemQuery, ItemSort,
    WeaponCategory,
};
use leptos::prelude::*;

#[server]
pub async fn get_items(query: ItemQuery) -> Result<Vec<Item>, ServerFnError> {
    use sqlx::SqlitePool;

    let pool = expect_context::<SqlitePool>();
    crate::db::list_items(&pool, &query).await.map_err(|err| ServerFnError::new(err.to_string()))
}

#[server]
pub async fn get_item_sources() -> Result<Vec<String>, ServerFnError> {
    use sqlx::SqlitePool;

    let pool = expect_context::<SqlitePool>();
    crate::db::list_item_sources(&pool).await.map_err(|err| ServerFnError::new(err.to_string()))
}

#[server]
pub async fn get_item_types() -> Result<Vec<String>, ServerFnError> {
    use sqlx::SqlitePool;

    let pool = expect_context::<SqlitePool>();
    crate::db::list_item_types(&pool).await.map_err(|err| ServerFnError::new(err.to_string()))
}

#[server]
pub async fn get_item_rarities() -> Result<Vec<String>, ServerFnError> {
    use sqlx::SqlitePool;

    let pool = expect_context::<SqlitePool>();
    crate::db::list_item_rarities(&pool).await.map_err(|err| ServerFnError::new(err.to_string()))
}

#[server]
pub async fn get_item_properties() -> Result<Vec<ItemPropertyOption>, ServerFnError> {
    use sqlx::SqlitePool;

    let pool = expect_context::<SqlitePool>();
    crate::db::list_item_properties(&pool).await.map_err(|err| ServerFnError::new(err.to_string()))
}

#[component]
pub fn ItemsPage() -> impl IntoView {
    let search = RwSignal::new(String::new());
    let selected_kinds = RwSignal::new(Vec::<ItemKind>::new());
    let selected_sources = RwSignal::new(Vec::<String>::new());
    let selected_types = RwSignal::new(Vec::<String>::new());
    let selected_rarities = RwSignal::new(Vec::<String>::new());
    let selected_properties = RwSignal::new(Vec::<ItemPropertyOption>::new());
    let selected_weapon_categories = RwSignal::new(Vec::<WeaponCategory>::new());
    let selected_damage_types = RwSignal::new(Vec::<DamageType>::new());
    let selected_attunement = RwSignal::new(Vec::<AttunementRequirement>::new());
    let sort = RwSignal::new(ItemSort::default());
    let selected_item = RwSignal::new(None::<Item>);

    let draft_kinds = RwSignal::new(Vec::<ItemKind>::new());
    let draft_sources = RwSignal::new(Vec::<String>::new());
    let draft_types = RwSignal::new(Vec::<String>::new());
    let draft_rarities = RwSignal::new(Vec::<String>::new());
    let draft_properties = RwSignal::new(Vec::<ItemPropertyOption>::new());
    let draft_weapon_categories = RwSignal::new(Vec::<WeaponCategory>::new());
    let draft_damage_types = RwSignal::new(Vec::<DamageType>::new());
    let draft_attunement = RwSignal::new(Vec::<AttunementRequirement>::new());

    let filters_dialog = NodeRef::<leptos::html::Dialog>::new();

    let items = Resource::new(
        move || ItemQuery {
            search: search.get(),
            sources: selected_sources.get(),
            kinds: selected_kinds.get(),
            types: selected_types.get(),
            rarities: selected_rarities.get(),
            property_codes: selected_properties.get().iter().map(|option| option.code.clone()).collect(),
            weapon_categories: selected_weapon_categories.get(),
            damage_types: selected_damage_types.get(),
            attunement: selected_attunement.get(),
            sort: sort.get(),
        },
        |query| async move { get_items(query).await },
    );

    let available_sources = Resource::new(|| (), |_| async move { get_item_sources().await });
    let available_types = Resource::new(|| (), |_| async move { get_item_types().await });
    let available_rarities = Resource::new(|| (), |_| async move { get_item_rarities().await });
    let available_properties = Resource::new(|| (), |_| async move { get_item_properties().await });

    let open_filters = move || {
        draft_kinds.set(selected_kinds.get());
        draft_sources.set(selected_sources.get());
        draft_types.set(selected_types.get());
        draft_rarities.set(selected_rarities.get());
        draft_properties.set(selected_properties.get());
        draft_weapon_categories.set(selected_weapon_categories.get());
        draft_damage_types.set(selected_damage_types.get());
        draft_attunement.set(selected_attunement.get());
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
        selected_kinds.set(draft_kinds.get());
        selected_sources.set(draft_sources.get());
        selected_types.set(draft_types.get());
        selected_rarities.set(draft_rarities.get());
        selected_properties.set(draft_properties.get());
        selected_weapon_categories.set(draft_weapon_categories.get());
        selected_damage_types.set(draft_damage_types.get());
        selected_attunement.set(draft_attunement.get());
        if let Some(dialog) = filters_dialog.get() {
            dialog.close();
        }
    };

    let toggle_name_sort = move |_| {
        sort.update(|s| *s = if *s == ItemSort::NameAsc { ItemSort::NameDesc } else { ItemSort::NameAsc })
    };
    let toggle_source_sort = move |_| {
        sort.update(|s| *s = if *s == ItemSort::SourceAsc { ItemSort::SourceDesc } else { ItemSort::SourceAsc })
    };
    let toggle_rarity_sort = move |_| {
        sort.update(|s| *s = if *s == ItemSort::RarityAsc { ItemSort::RarityDesc } else { ItemSort::RarityAsc })
    };
    let toggle_value_sort = move |_| {
        sort.update(|s| *s = if *s == ItemSort::ValueAsc { ItemSort::ValueDesc } else { ItemSort::ValueAsc })
    };
    let toggle_weight_sort = move |_| {
        sort.update(|s| *s = if *s == ItemSort::WeightAsc { ItemSort::WeightDesc } else { ItemSort::WeightAsc })
    };

    let active_filter_count = move || {
        selected_kinds.get().len()
            + selected_sources.get().len()
            + selected_types.get().len()
            + selected_rarities.get().len()
            + selected_properties.get().len()
            + selected_weapon_categories.get().len()
            + selected_damage_types.get().len()
            + selected_attunement.get().len()
    };

    view! {
        <div class="flex flex-col gap-4">
            <a href="/compendium" class="btn btn-ghost btn-sm w-fit gap-2">
                "← Back to Compendium"
            </a>
            <h1 class="text-3xl">"Items"</h1>

            <div class="flex flex-wrap gap-2 items-center">
                <SearchBox value=search placeholder="Search items..." />
                {filter_button(active_filter_count, open_filters)}
            </div>

            {filter_dialog(
                filters_dialog,
                "Filter Items",
                close_filters,
                save_filters,
                vec![
                    filter_chip_group(
                            "Kind",
                            FilterOptions::Static(ItemKind::ALL.to_vec()),
                            |kind: &ItemKind| kind.label().to_string(),
                            draft_kinds,
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
                            "Type",
                            FilterOptions::Async(available_types),
                            |item_type: &String| item_type.clone(),
                            draft_types,
                        )
                        .into_any(),
                    filter_chip_group(
                            "Rarity",
                            FilterOptions::Async(available_rarities),
                            |rarity: &String| rarity_label(rarity),
                            draft_rarities,
                        )
                        .into_any(),
                    filter_chip_group(
                            "Property",
                            FilterOptions::Async(available_properties),
                            |property: &ItemPropertyOption| property.label.clone(),
                            draft_properties,
                        )
                        .into_any(),
                    filter_chip_group(
                            "Weapon Category",
                            FilterOptions::Static(WeaponCategory::ALL.to_vec()),
                            |category: &WeaponCategory| category.label().to_string(),
                            draft_weapon_categories,
                        )
                        .into_any(),
                    filter_chip_group(
                            "Damage Type",
                            FilterOptions::Static(DamageType::ALL.to_vec()),
                            |damage_type: &DamageType| damage_type.label().to_string(),
                            draft_damage_types,
                        )
                        .into_any(),
                    filter_chip_group(
                            "Attunement",
                            FilterOptions::Static(AttunementRequirement::ALL.to_vec()),
                            |attunement: &AttunementRequirement| attunement.label().to_string(),
                            draft_attunement,
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
                                    {move || sort_indicator(sort.get(), ItemSort::NameAsc, ItemSort::NameDesc)}
                                </th>
                                <th>"Type"</th>
                                <th class="cursor-pointer select-none" on:click=toggle_rarity_sort>
                                    "Rarity"
                                    {move || sort_indicator(sort.get(), ItemSort::RarityAsc, ItemSort::RarityDesc)}
                                </th>
                                <th class="cursor-pointer select-none" on:click=toggle_value_sort>
                                    "Cost"
                                    {move || sort_indicator(sort.get(), ItemSort::ValueAsc, ItemSort::ValueDesc)}
                                </th>
                                <th class="cursor-pointer select-none" on:click=toggle_weight_sort>
                                    "Weight"
                                    {move || sort_indicator(sort.get(), ItemSort::WeightAsc, ItemSort::WeightDesc)}
                                </th>
                                <th>"Attunement"</th>
                                <th class="cursor-pointer select-none" on:click=toggle_source_sort>
                                    "Source"
                                    {move || sort_indicator(sort.get(), ItemSort::SourceAsc, ItemSort::SourceDesc)}
                                </th>
                            </tr>
                        </thead>
                        <tbody>
                            <Suspense fallback=move || {
                                view! {
                                    <tr>
                                        <td colspan="7">"Loading items..."</td>
                                    </tr>
                                }
                            }>
                                {move || Suspend::new(async move {
                                    match items.await {
                                        Ok(rows) if rows.is_empty() => {
                                            view! {
                                                <tr>
                                                    <td colspan="7">"No items match your filters."</td>
                                                </tr>
                                            }
                                                .into_any()
                                        }
                                        Ok(rows) => {
                                            rows.into_iter()
                                                .map(|item| {
                                                    let row_item = item.clone();
                                                    view! {
                                                        <tr
                                                            class="row-hover cursor-pointer"
                                                            on:click=move |_| { selected_item.set(Some(row_item.clone())) }
                                                        >
                                                            <td>{item.name.clone()}</td>
                                                            <td>{item.item_type_label.clone().unwrap_or_else(|| "\u{2014}".to_string())}</td>
                                                            <td>{rarity_label(&item.rarity)}</td>
                                                            <td>{value_label(item.value_cp)}</td>
                                                            <td>{weight_label(item.weight_lb)}</td>
                                                            <td>{attunement_badge(&item)}</td>
                                                            <td>{item.source.clone()}</td>
                                                        </tr>
                                                    }
                                                })
                                                .collect_view()
                                                .into_any()
                                        }
                                        Err(err) => {
                                            view! {
                                                <tr>
                                                    <td colspan="7">{format!("Failed to load items: {err}")}</td>
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
                    {move || selected_item.get().map(|item| view! { <ItemDetailCard item=item /> })}
                </div>
            </div>
        </div>
    }
}

/// 5etools rarity strings are already lowercase words ("very rare",
/// "unknown (magic)") — this just title-cases each word for display.
fn rarity_label(rarity: &str) -> String {
    rarity
        .split(' ')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Converts a copper-piece value into the largest sensible denomination
/// ("15 gp", "2 sp", "8 cp") rather than always showing raw copper.
fn value_label(value_cp: Option<f64>) -> String {
    let Some(cp) = value_cp else { return "\u{2014}".to_string() };
    let gp = cp / 100.0;
    if gp >= 1.0 {
        return format_denomination(gp, "gp");
    }
    let sp = cp / 10.0;
    if sp >= 1.0 {
        return format_denomination(sp, "sp");
    }
    format_denomination(cp, "cp")
}

fn format_denomination(amount: f64, unit: &str) -> String {
    if amount.fract() == 0.0 { format!("{} {unit}", amount as i64) } else { format!("{amount:.2} {unit}") }
}

fn weight_label(weight_lb: Option<f64>) -> String {
    match weight_lb {
        None => "\u{2014}".to_string(),
        Some(weight) if weight.fract() == 0.0 => format!("{} lb.", weight as i64),
        Some(weight) => format!("{weight} lb."),
    }
}

fn attunement_badge(item: &Item) -> impl IntoView {
    item.requires_attunement
        .then(|| view! { <span class="badge badge-outline badge-sm font-normal">"Attunement"</span> })
}

/// Compact weapon/armor stat summary for the detail card — omitted entirely
/// for items with none of these fields (most wondrous items, tools, gear).
fn combat_stats_line(item: &Item) -> Option<String> {
    let mut parts = Vec::new();

    if let Some(damage) = &item.damage {
        let dice = match &damage.dice_versatile {
            Some(versatile) => format!("{} / {}", damage.dice, versatile),
            None => damage.dice.clone(),
        };
        let damage_type = damage.damage_type.map(|d| format!(" {}", d.label())).unwrap_or_default();
        parts.push(format!("Damage: {dice}{damage_type}"));
    }
    if let Some(ac) = item.base_ac {
        parts.push(format!("AC {ac}"));
    }
    if let Some(range) = &item.range {
        parts.push(match range.long {
            Some(long) => format!("Range {}/{long} ft.", range.normal),
            None => format!("Range {} ft.", range.normal),
        });
    }
    if let Some(category) = item.weapon_category {
        parts.push(category.label().to_string());
    }
    if !item.properties.is_empty() {
        parts.push(item.properties.iter().map(|property| property.label.clone()).collect::<Vec<_>>().join(", "));
    }

    if parts.is_empty() { None } else { Some(parts.join(" \u{00B7} ")) }
}

fn magic_bonuses_line(bonuses: &ItemMagicBonuses) -> Option<String> {
    if bonuses.is_empty() {
        return None;
    }
    let mut parts = Vec::new();
    if let Some(v) = bonuses.weapon_attack_and_damage {
        parts.push(format!("+{v} to attack and damage rolls"));
    }
    if let Some(v) = bonuses.weapon_attack {
        parts.push(format!("+{v} to attack rolls"));
    }
    if let Some(v) = bonuses.weapon_damage {
        parts.push(format!("+{v} to damage rolls"));
    }
    if let Some(v) = bonuses.ac {
        parts.push(format!("+{v} to AC"));
    }
    if let Some(v) = bonuses.spell_attack {
        parts.push(format!("+{v} to spell attack rolls"));
    }
    if let Some(v) = bonuses.spell_save_dc {
        parts.push(format!("+{v} to spell save DC"));
    }
    if let Some(v) = bonuses.saving_throw {
        parts.push(format!("+{v} to saving throws"));
    }
    if let Some(v) = bonuses.ability_check {
        parts.push(format!("+{v} to ability checks"));
    }
    if let Some(v) = bonuses.proficiency_bonus {
        parts.push(format!("+{v} to proficiency bonus"));
    }
    Some(parts.join(", "))
}

fn charges_line(item: &Item) -> Option<String> {
    let charges = item.charges.as_ref()?;
    let trigger = charges.recharge_trigger.as_deref().unwrap_or("");
    let amount = charges.recharge_amount.as_deref().unwrap_or("");
    Some(format!(
        "Charges: {}{}{}",
        charges.max,
        if trigger.is_empty() { String::new() } else { format!(", recharges {trigger}") },
        if amount.is_empty() { String::new() } else { format!(" ({amount})") },
    ))
}

#[component]
fn ItemDetailCard(item: Item) -> impl IntoView {
    let body = item.entries.iter().map(entry_view).collect_view();
    let combat_stats = combat_stats_line(&item);
    let magic_bonuses = magic_bonuses_line(&item.magic_bonuses);
    let charges = charges_line(&item);

    view! {
        <div class="card bg-base-100 shadow-xl border border-base-300">
            <div class="card-body">
                <h2 class="card-title">
                    {item.name.clone()}
                    <span class="badge badge-outline badge-sm font-normal">{item.source.clone()}</span>
                    {item.is_group.then(|| view! { <span class="badge badge-secondary badge-sm font-normal">"Item Group"</span> })}
                </h2>
                <p class="text-sm opacity-70">
                    {item.item_type_label.clone().unwrap_or_else(|| "\u{2014}".to_string())}
                    " \u{00B7} "
                    {rarity_label(&item.rarity)}
                </p>
                <p class="text-sm opacity-70">
                    {format!("Cost: {} \u{00B7} Weight: {}", value_label(item.value_cp), weight_label(item.weight_lb))}
                </p>
                {(item.requires_attunement || item.attunement_note.is_some())
                    .then(|| {
                        let note = item.attunement_note.clone();
                        view! {
                            <p class="text-sm opacity-70 italic">
                                {match note {
                                    Some(note) => format!("Requires attunement {note}"),
                                    None => "Requires attunement".to_string(),
                                }}
                            </p>
                        }
                    })}
                {combat_stats.map(|line| view! { <p class="text-sm">{line}</p> })}
                {magic_bonuses.map(|line| view! { <p class="text-sm">{line}</p> })}
                {charges.map(|line| view! { <p class="text-sm">{line}</p> })}
                {item
                    .is_group
                    .then(|| {
                        view! {
                            <ul class="list-disc pl-5 text-sm">
                                {item
                                    .group_members
                                    .iter()
                                    .map(|member| view! { <li>{member.name.clone()}</li> })
                                    .collect_view()}
                            </ul>
                        }
                    })}
                {(!item.is_group && !item.member_of_groups.is_empty())
                    .then(|| view! { <p class="text-sm opacity-70">{format!("Part of: {}", item.member_of_groups.join(", "))}</p> })}
                <div class="divider my-1"></div>
                <div class="space-y-2 text-sm">{body}</div>
            </div>
        </div>
    }
}

fn sort_indicator(current: ItemSort, asc: ItemSort, desc: ItemSort) -> &'static str {
    if current == asc {
        " \u{25B2}"
    } else if current == desc {
        " \u{25BC}"
    } else {
        ""
    }
}
