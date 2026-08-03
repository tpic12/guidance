mod abilities;
mod asi;
mod background;
mod basics;
mod class;
mod languages;
mod optional_features;
mod review;
mod skills;
mod species;
mod spells;
mod tools;

pub use abilities::abilities_step;
pub use asi::asi_step;
pub use background::background_step;
pub use basics::basics_step;
pub use class::class_step;
pub use languages::languages_step;
pub use optional_features::{optional_features_step, OptionalFeaturePoolResource};
pub use review::{review_step, ReviewSummary};
pub use skills::skills_step;
pub use species::species_step;
pub use spells::spells_step;
pub use tools::tools_step;

use leptos::prelude::*;

// Shared empty state for the Background/Species/Spells/ASI/Optional
// Features steps' right-side detail panel, before anything's been
// hovered/selected yet.
fn detail_panel_placeholder(message: &'static str) -> impl IntoView {
    view! {
        <div class="card bg-base-200 border border-base-300 border-dashed">
            <div class="card-body items-center text-center py-8">
                <p class="text-sm opacity-60">{message}</p>
            </div>
        </div>
    }
}
