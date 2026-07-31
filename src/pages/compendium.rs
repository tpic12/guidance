use leptos::prelude::*;
use leptos_meta::provide_meta_context;
use crate::components::card::Card;

#[derive(Clone, Debug, PartialEq)]
struct CompOption {
    key: String,
    label: String,
    source: String,
    icon: String,
    /// Card-catalog-style index code shown above the label on `Card`.
    code: &'static str
}

#[component]
pub fn CompendiumPage() -> impl IntoView {
    provide_meta_context();

    let comp_options = vec![
        CompOption {
            key: "Classes".to_string(),
            label: "Classes".to_string(),
            source: "/compendium/classes".to_string(),
            icon: include_str!("../assets/character_icon.svg").to_string(),
            code: "CLS"
        },
        CompOption {
            key: "Spells".to_string(),
            label: "Spells".to_string(),
            source: "/compendium/spells".to_string(),
            icon: include_str!("../assets/spell_icon.svg").to_string(),
            code: "SPL"
        },
        CompOption {
            key: "Items".to_string(),
            label: "Items".to_string(),
            source: "/compendium/items".to_string(),
            icon: include_str!("../assets/items_icon.svg").to_string(),
            code: "ITM"
        },
        CompOption {
            key: "Backgrounds".to_string(),
            label: "Backgrounds".to_string(),
            source: "/compendium/backgrounds".to_string(),
            icon: include_str!("../assets/character_icon.svg").to_string(),
            code: "BKG"
        },
        CompOption {
            key: "Feats".to_string(),
            label: "Feats".to_string(),
            source: "/compendium/feats".to_string(),
            icon: include_str!("../assets/feats.svg").to_string(),
            code: "FEA"
        },
        CompOption {
            key: "Species".to_string(),
            label: "Species".to_string(),
            source: "/compendium/species".to_string(),
            icon: include_str!("../assets/characters_icon.svg").to_string(),
            code: "SPC"
        },
        CompOption {
            key: "Optional Features".to_string(),
            label: "Optional Features".to_string(),
            source: "/compendium/optional-features".to_string(),
            icon: include_str!("../assets/book_icon.svg").to_string(),
            code: "OPT"
        }
    ];

    view! {
        <div class="">
            <p class="font-mono text-xs tracking-[0.25em] text-primary/80 mb-2">"THE COMPENDIUM"</p>
            <h1 class="font-display text-3xl font-semibold mb-1">"Shared reference content"</h1>
            <p class="text-sm opacity-70 mb-8">
                "Rules and content imported for this table — pick a drawer to browse."
            </p>
            <div class="w-full grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-3 gap-4">
                <For
                    each=move || comp_options.clone()
                    key=|option| option.key.clone()
                    children=move |option| {
                        view! {
                            <Card
                                label=option.label
                                source=option.source
                                icon=option.icon
                                code=option.code
                            />
                        }
                    }
                />
            </div>
        </div>
    }
}