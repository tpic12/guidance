use crate::components::entry_view::entry_view;
use crate::models::species::Species;
use leptos::prelude::*;

#[component]
pub fn SpeciesDetailCard(species: Species) -> impl IntoView {
    let body = species.entries.iter().map(entry_view).collect_view();

    view! {
        <div class="card bg-base-100 shadow-xl border border-base-300">
            <div class="card-body">
                <h2 class="card-title">
                    {species.name.clone()}
                    <span class="badge badge-outline badge-sm font-normal">
                        {species.source.clone()}
                    </span>
                </h2>
                {species
                    .ability
                    .clone()
                    .map(|ability| {
                        view! { <p class="text-sm">{format!("Ability Scores: {ability}")}</p> }
                    })}
                {species
                    .size
                    .clone()
                    .map(|size| view! { <p class="text-sm">{format!("Size: {size}")}</p> })}
                {species
                    .speed
                    .clone()
                    .map(|speed| view! { <p class="text-sm">{format!("Speed: {speed}")}</p> })}
                {species
                    .darkvision
                    .map(|range| {
                        view! { <p class="text-sm">{format!("Darkvision: {range} ft.")}</p> }
                    })}
                <div class="divider my-1"></div>
                <div class="space-y-2 text-sm">{body}</div>
            </div>
        </div>
    }
}
