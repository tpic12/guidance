use crate::components::entry_view::entry_view;
use crate::models::feat::Feat;
use leptos::prelude::*;

#[component]
pub fn FeatDetailCard(feat: Feat) -> impl IntoView {
    let body = feat.entries.iter().map(entry_view).collect_view();

    view! {
        <div class="card bg-base-100 shadow-xl border border-base-300">
            <div class="card-body">
                <h2 class="card-title">
                    {feat.name.clone()}
                    <span class="badge badge-outline badge-sm font-normal">
                        {feat.source.clone()}
                    </span>
                </h2>
                {feat
                    .prerequisite
                    .clone()
                    .map(|prerequisite| {
                        view! {
                            <p class="text-sm opacity-70 italic">
                                {format!("Prerequisite: {prerequisite}")}
                            </p>
                        }
                    })}
                {feat.ability.clone().map(|ability| view! { <p class="text-sm">{ability}</p> })}
                <div class="divider my-1"></div>
                <div class="space-y-2 text-sm">{body}</div>
            </div>
        </div>
    }
}
