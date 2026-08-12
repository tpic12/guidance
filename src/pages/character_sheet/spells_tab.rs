use super::badges::granted_badge;
use crate::models::spell::Spell;
use leptos::prelude::*;
use std::collections::BTreeMap;

#[component]
pub(super) fn SpellsTab(
    casting_lines: Vec<String>,
    slot_lines: Vec<String>,
    pact_slot_lines: Vec<String>,
    all_castables: Vec<(Spell, bool)>,
) -> impl IntoView {
    if casting_lines.is_empty() && slot_lines.is_empty() && pact_slot_lines.is_empty() && all_castables.is_empty() {
        return view! { <p class="opacity-70 text-sm">"No spells known."</p> }.into_any();
    }

    let mut by_level: BTreeMap<u8, Vec<(Spell, bool)>> = BTreeMap::new();
    for (spell, is_granted) in all_castables {
        by_level.entry(spell.level).or_default().push((spell, is_granted));
    }

    view! {
        <div class="flex flex-col gap-3">
            {casting_lines.into_iter().map(|line| view! { <p class="text-sm">{line}</p> }).collect_view()}
            {(!slot_lines.is_empty())
                .then(|| view! { <p class="text-sm">{format!("Slots — {}", slot_lines.join(", "))}</p> })}
            {(!pact_slot_lines.is_empty())
                .then(|| {
                    view! {
                        <p class="text-sm">{format!("Pact Magic slots — {}", pact_slot_lines.join(", "))}</p>
                    }
                })}
            {by_level
                .into_iter()
                .map(|(level, spells)| {
                    let heading = if level == 0 { "Cantrips".to_string() } else { format!("Level {level}") };
                    view! {
                        <div class="flex flex-col gap-1">
                            <h3 class="font-mono text-xs tracking-[0.15em] uppercase text-primary/80">{heading}</h3>
                            {spells
                                .into_iter()
                                .map(|(spell, is_granted)| {
                                    view! {
                                        <p class="text-sm flex flex-wrap items-center gap-2">
                                            <span>{format!("{} ({})", spell.name, spell.school.label())}</span>
                                            {is_granted.then(granted_badge)}
                                        </p>
                                    }
                                })
                                .collect_view()}
                        </div>
                    }
                })
                .collect_view()}
        </div>
    }
        .into_any()
}
