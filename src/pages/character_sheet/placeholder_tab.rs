use leptos::prelude::*;

/// Empty-state tab body — reused by any tab with no data behind it yet
/// (Inventory, Notes).
#[component]
pub(super) fn PlaceholderTab(title: &'static str, body: &'static str) -> impl IntoView {
    view! {
        <div class="card bg-base-100 border border-base-300">
            <div class="card-body items-center text-center py-10 gap-2">
                <p class="font-display text-lg opacity-80">{title}</p>
                <p class="text-sm opacity-60 max-w-md">{body}</p>
            </div>
        </div>
    }
}
