use crate::models::entry::Entry;
use leptos::prelude::*;

/// Recursively renders one block of rules text ([`Entry`]) to a view. Shared
/// by the class detail page and the spell detail card.
pub fn entry_view(entry: &Entry) -> AnyView {
    match entry {
        Entry::Text { text } => view! { <p>{text.clone()}</p> }.into_any(),
        Entry::List { items } => view! {
            <ul class="list-disc pl-5 space-y-1">
                {items.iter().map(|item| view! { <li>{list_item_view(item)}</li> }).collect_view()}
            </ul>
        }
        .into_any(),
        Entry::Item { name, entries } => view! {
            <p>
                <span class="font-semibold">{format!("{name}. ")}</span>
                {entries_plain_text(entries)}
            </p>
        }
        .into_any(),
        Entry::Section { name, entries } => view! {
            <div class="space-y-2">
                {name.clone().map(|name| view! { <h4 class="font-semibold">{name}</h4> })}
                {entries.iter().map(entry_view).collect_view()}
            </div>
        }
        .into_any(),
        Entry::Table { caption, col_labels, rows } => view! {
            <div class="overflow-x-auto">
                <table class="table table-sm table-zebra w-auto">
                    {caption
                        .clone()
                        .map(|caption| {
                            view! {
                                <caption class="text-left font-semibold pb-1">{caption}</caption>
                            }
                        })} <thead>
                        <tr>
                            {col_labels
                                .iter()
                                .map(|label| view! { <th>{label.clone()}</th> })
                                .collect_view()}
                        </tr>
                    </thead>
                    <tbody>
                        {rows
                            .iter()
                            .map(|row| {
                                view! {
                                    <tr>
                                        {row
                                            .iter()
                                            .map(|cell| view! { <td>{cell.clone()}</td> })
                                            .collect_view()}
                                    </tr>
                                }
                            })
                            .collect_view()}
                    </tbody>
                </table>
            </div>
        }
        .into_any(),
    }
}

/// List items render inline (no nested <p> inside <li>).
fn list_item_view(entry: &Entry) -> AnyView {
    match entry {
        Entry::Text { text } => text.clone().into_any(),
        Entry::Item { name, entries } => view! {
            <span>
                <span class="font-semibold">{format!("{name}. ")}</span>
                {entries_plain_text(entries)}
            </span>
        }
        .into_any(),
        other => entry_view(other),
    }
}

fn entries_plain_text(entries: &[Entry]) -> String {
    entries
        .iter()
        .filter_map(|entry| match entry {
            Entry::Text { text } => Some(text.clone()),
            Entry::Section { entries, .. } | Entry::List { items: entries } => {
                Some(entries_plain_text(entries))
            }
            Entry::Item { name, entries } => {
                Some(format!("{name}. {}", entries_plain_text(entries)))
            }
            Entry::Table { .. } => None,
        })
        .collect::<Vec<_>>()
        .join(" ")
}
