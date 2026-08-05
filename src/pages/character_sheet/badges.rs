use leptos::prelude::*;

/// Cantrip/level marker shown next to a spell's name — shared by the Actions
/// tab's expandable rows and the Spells tab's per-level lists.
pub(super) fn level_badge(level: u8) -> impl IntoView {
    let label = if level == 0 { "Cantrip".to_string() } else { format!("L{level}") };
    view! { <span class="badge badge-sm">{label}</span> }
}

/// Marks a spell as a subclass/feat grant (always prepared, not a player pick).
pub(super) fn granted_badge() -> impl IntoView {
    view! { <span class="badge badge-sm badge-outline">"Granted"</span> }
}
