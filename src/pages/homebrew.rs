use leptos::prelude::*;

#[component]
pub fn HomebrewPage() -> impl IntoView {
    view! {
        <div>
            <p class="font-mono text-xs tracking-[0.25em] text-primary/80 mb-2">"HOMEBREW"</p>
            <h1 class="font-display text-3xl font-semibold mb-1">"Your table's own content"</h1>
            <p class="text-sm opacity-70">
                "Authoring tools for homebrew classes, spells, and items aren't built yet — check back soon."
            </p>
        </div>
    }
}