use super::badges::{granted_badge, level_badge};
use super::tab_link;
use crate::components::entry_view::entry_view;
use crate::models::spell::{CastingType, Spell};
use leptos::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActionFilter {
    All,
    Action,
    BonusAction,
    Reaction,
}

impl ActionFilter {
    const ALL: [ActionFilter; 4] =
        [ActionFilter::All, ActionFilter::Action, ActionFilter::BonusAction, ActionFilter::Reaction];

    fn label(&self) -> &'static str {
        match self {
            ActionFilter::All => "All",
            ActionFilter::Action => "Action",
            ActionFilter::BonusAction => "Bonus Action",
            ActionFilter::Reaction => "Reaction",
        }
    }

    fn matches(&self, casting_type: CastingType) -> bool {
        match self {
            ActionFilter::All => true,
            ActionFilter::Action => casting_type == CastingType::Action,
            ActionFilter::BonusAction => casting_type == CastingType::BonusAction,
            ActionFilter::Reaction => casting_type == CastingType::Reaction,
        }
    }
}

#[component]
pub(super) fn ActionsTab(all_castables: Vec<(Spell, bool)>) -> impl IntoView {
    let filter = RwSignal::new(ActionFilter::All);
    let panels: Vec<(ActionFilter, AnyView)> = ActionFilter::ALL
        .iter()
        .map(|&f| {
            let rows: Vec<AnyView> = all_castables
                .iter()
                .filter(|(spell, _)| f.matches(spell.casting_type))
                .map(|(spell, is_granted)| action_row(spell, *is_granted).into_any())
                .collect();
            let content = if rows.is_empty() {
                view! { <p class="opacity-70 text-sm">{format!("No {} spells.", f.label().to_lowercase())}</p> }
                    .into_any()
            } else {
                view! { <div class="flex flex-col gap-2">{rows}</div> }.into_any()
            };
            (f, content)
        })
        .collect();

    view! {
        <div class="flex flex-col gap-3">
            <div role="tablist" class="tabs tabs-boxed tabs-sm w-fit flex-wrap">
                {ActionFilter::ALL.iter().map(|&f| tab_link(f.label(), f, filter)).collect_view()}
            </div>
            {panels
                .into_iter()
                .map(|(f, content)| view! { <div class:hidden=move || filter.get() != f>{content}</div> })
                .collect_view()}
        </div>
    }
}

fn action_row(spell: &Spell, is_granted: bool) -> impl IntoView {
    let condition = spell.casting_condition.clone();
    let name = spell.name.clone();
    let entries = spell.description.clone();
    let level = spell.level;
    view! {
        <div class="collapse collapse-arrow bg-base-100 border border-base-300">
            <input type="checkbox" />
            <div class="collapse-title flex flex-wrap items-center gap-2 py-2 min-h-0">
                <span class="font-sans font-medium">{name}</span>
                {level_badge(level)}
                {is_granted.then(granted_badge)}
                {condition.map(|condition| view! { <span class="text-xs opacity-70">{condition}</span> })}
            </div>
            <div class="collapse-content text-sm space-y-2">
                {entries.iter().map(entry_view).collect_view()}
            </div>
        </div>
    }
}
