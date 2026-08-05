use super::tab_link;
use leptos::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FeatureTab {
    Summary,
    Species,
    Class,
    Subclass,
    Feats,
    Optional,
}

#[component]
pub(super) fn FeaturesTab(
    summary: AnyView,
    species: Option<AnyView>,
    class_sections: Vec<AnyView>,
    subclass_sections: Vec<AnyView>,
    feats: Option<AnyView>,
    optional: Option<AnyView>,
) -> impl IntoView {
    let active = RwSignal::new(FeatureTab::Summary);
    let has_species = species.is_some();
    let has_class = !class_sections.is_empty();
    let has_subclass = !subclass_sections.is_empty();
    let has_feats = feats.is_some();
    let has_optional = optional.is_some();

    view! {
        <div class="flex flex-col gap-3">
            <div role="tablist" class="tabs tabs-boxed tabs-sm w-fit flex-wrap">
                {tab_link("Summary", FeatureTab::Summary, active)}
                {has_species.then(|| tab_link("Species", FeatureTab::Species, active))}
                {has_class.then(|| tab_link("Class", FeatureTab::Class, active))}
                {has_subclass.then(|| tab_link("Subclass", FeatureTab::Subclass, active))}
                {has_feats.then(|| tab_link("Feats", FeatureTab::Feats, active))}
                {has_optional.then(|| tab_link("Optional", FeatureTab::Optional, active))}
            </div>
            <div class:hidden=move || active.get() != FeatureTab::Summary>{summary}</div>
            <div class:hidden=move || active.get() != FeatureTab::Species>{species}</div>
            <div class="flex flex-col gap-3" class:hidden=move || active.get() != FeatureTab::Class>
                {class_sections.into_iter().collect_view()}
            </div>
            <div class="flex flex-col gap-3" class:hidden=move || active.get() != FeatureTab::Subclass>
                {subclass_sections.into_iter().collect_view()}
            </div>
            <div class:hidden=move || active.get() != FeatureTab::Feats>{feats}</div>
            <div class:hidden=move || active.get() != FeatureTab::Optional>{optional}</div>
        </div>
    }
}
