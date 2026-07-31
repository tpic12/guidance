use crate::components::entry_view::entry_view;
use crate::models::background::Background;
use leptos::prelude::*;

#[component]
pub fn BackgroundDetailCard(background: Background) -> impl IntoView {
    // The entries already open with the skill/language/equipment summary list
    // from the source data, so the card doesn't repeat the proficiencies.
    let body = background.entries.iter().map(entry_view).collect_view();

    view! {
        <div class="card bg-base-100 shadow-xl border border-base-300">
            <div class="card-body">
                <h2 class="card-title">
                    {background.name.clone()}
                    <span class="badge badge-outline badge-sm font-normal">
                        {background.source.clone()}
                    </span>
                </h2>
                <div class="space-y-2 text-sm">{body}</div>
            </div>
        </div>
    }
}
