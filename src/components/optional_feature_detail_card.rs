use crate::components::entry_view::entry_view;
use crate::models::optional_feature::{prerequisite_label, FeatureType, OptionalFeature};
use leptos::prelude::*;

/// One badge per distinct group (`group_label`), not per raw `FeatureType` —
/// an entry like "Two-Weapon Fighting" carries 3 Fighting Style variants
/// (Fighter/Bard/Ranger) that would otherwise render as 3 near-identical
/// badges here.
pub fn type_badges(feature_types: &[FeatureType]) -> impl IntoView {
    let mut labels: Vec<&'static str> = feature_types.iter().map(|feature_type| feature_type.group_label()).collect();
    labels.sort_unstable();
    labels.dedup();
    labels
        .into_iter()
        .map(|label| {
            view! { <span class="badge badge-outline badge-sm font-normal mr-1">{label}</span> }
        })
        .collect_view()
}

#[component]
pub fn OptionalFeatureDetailCard(feature: OptionalFeature) -> impl IntoView {
    let body = feature.entries.iter().map(entry_view).collect_view();
    let prerequisite = prerequisite_label(&feature.prerequisites);
    let consumes = feature.consumes.as_ref().map(|cost| {
        let amount = match (cost.amount, cost.amount_min, cost.amount_max) {
            (Some(amount), _, _) => format!("{amount} "),
            (None, Some(min), Some(max)) => format!("{min}-{max} "),
            (None, Some(min), None) => format!("{min}+ "),
            _ => String::new(),
        };
        format!("Costs: {amount}{}", cost.name)
    });

    view! {
        <div class="card bg-base-100 shadow-xl border border-base-300">
            <div class="card-body">
                <h2 class="card-title">
                    {feature.name.clone()}
                    <span class="badge badge-outline badge-sm font-normal">
                        {feature.source.clone()}
                    </span>
                </h2>
                <div>{type_badges(&feature.feature_types)}</div>
                {prerequisite
                    .map(|prerequisite| {
                        view! {
                            <p class="text-sm opacity-70 italic">
                                {format!("Prerequisite: {prerequisite}")}
                            </p>
                        }
                    })}
                {consumes.map(|line| view! { <p class="text-sm">{line}</p> })}
                <div class="divider my-1"></div>
                <div class="space-y-2 text-sm">{body}</div>
            </div>
        </div>
    }
}
