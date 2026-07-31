use crate::components::entry_view::entry_view;
use crate::models::spell::{CastingType, Components, Duration, Range, Spell};
use leptos::prelude::*;

#[component]
pub fn SpellDetailCard(spell: Spell) -> impl IntoView {
    // "At Higher Levels" blocks are named sections in the source data, so they
    // bring their own heading; just render them after the description.
    let body = spell
        .description
        .iter()
        .chain(spell.higher_level.iter())
        .map(entry_view)
        .collect_view();

    view! {
        <div class="card bg-base-100 shadow-xl border border-base-300">
            <div class="card-body">
                <h2 class="card-title">
                    {spell.name.clone()}
                    <span class="badge badge-outline badge-sm font-normal">
                        {spell.source.clone()}
                    </span>
                </h2>
                <p class="text-sm opacity-70 italic">
                    {level_school_label(&spell)} {if spell.ritual { " (ritual)" } else { "" }}
                </p>
                <ul class="text-sm space-y-1 my-2">
                    <li>
                        <span class="font-semibold">"Casting Time: "</span>
                        {casting_time_label(&spell)}
                    </li>
                    <li>
                        <span class="font-semibold">"Range: "</span>
                        {range_label(&spell.range)}
                    </li>
                    <li>
                        <span class="font-semibold">"Components: "</span>
                        {components_label(&spell.components)}
                    </li>
                    <li>
                        <span class="font-semibold">"Duration: "</span>
                        {duration_label(&spell.duration)}
                    </li>
                </ul>
                <div class="divider my-1"></div>
                <div class="space-y-2 text-sm">{body}</div>
            </div>
        </div>
    }
}

pub fn level_school_label(spell: &Spell) -> String {
    if spell.level == 0 {
        format!("Cantrip, {}", spell.school.label().to_lowercase())
    } else {
        format!("{} {}", ordinal(spell.level), spell.school.label().to_lowercase())
    }
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
    format!("{n}{suffix}-level")
}

pub fn casting_time_label(spell: &Spell) -> String {
    let unit = match spell.casting_type {
        CastingType::Action => "action",
        CastingType::BonusAction => "bonus action",
        CastingType::Reaction => "reaction",
        CastingType::Minute => "minute",
        CastingType::Hour => "hour",
        CastingType::Day => "day",
        CastingType::Week => "week",
        CastingType::Month => "month",
    };
    let base = if spell.casting_time == 1 {
        format!("1 {unit}")
    } else {
        format!("{} {unit}s", spell.casting_time)
    };
    match &spell.casting_condition {
        Some(condition) => format!("{base}, {condition}"),
        None => base,
    }
}

pub fn range_label(range: &Range) -> String {
    match (&range.unit, range.distance) {
        (Some(unit), Some(distance)) => format!("{distance} {unit} ({})", range.kind),
        _ => range.kind.replace('_', " "),
    }
}

pub fn components_label(components: &Components) -> String {
    let mut parts = Vec::new();
    if components.verbal {
        parts.push("V".to_string());
    }
    if components.somatic {
        parts.push("S".to_string());
    }
    if let Some(material) = &components.material {
        parts.push(format!("M ({material})"));
    }
    if parts.is_empty() { "None".to_string() } else { parts.join(", ") }
}

pub fn duration_label(duration: &Duration) -> String {
    let base = match (&duration.unit, duration.amount) {
        (Some(unit), Some(amount)) => format!("{amount} {unit}"),
        _ => duration.kind.replace('_', " "),
    };
    if duration.concentration { format!("Concentration, up to {base}") } else { base }
}
