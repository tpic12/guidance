//! Shared helpers for turning 5etools-shaped rules text into [`Entry`] trees,
//! used by both the spell and class transforms.

use crate::models::entry::Entry;
use serde_json::Value;

pub(crate) fn entries_from_values(values: &[Value]) -> Vec<Entry> {
    values.iter().filter_map(entry_from_value).collect()
}

pub(crate) fn entry_from_value(value: &Value) -> Option<Entry> {
    let obj = match value {
        Value::String(text) => return Some(Entry::Text { text: clean_tags(text) }),
        Value::Object(obj) => obj,
        _ => return None,
    };

    match obj.get("type").and_then(Value::as_str).unwrap_or("") {
        "list" => Some(Entry::List {
            items: entries_from_values(obj.get("items").and_then(Value::as_array)?),
        }),
        "item" => Some(Entry::Item {
            name: clean_tags(obj.get("name").and_then(Value::as_str).unwrap_or_default()),
            entries: item_entries(obj),
        }),
        "entries" | "inset" => Some(Entry::Section {
            name: obj.get("name").and_then(Value::as_str).map(clean_tags),
            entries: entries_from_values(
                obj.get("entries").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[]),
            ),
        }),
        "table" => Some(Entry::Table {
            caption: obj.get("caption").and_then(Value::as_str).map(clean_tags),
            col_labels: obj
                .get("colLabels")
                .and_then(Value::as_array)
                .map(|labels| {
                    labels.iter().filter_map(Value::as_str).map(clean_tags).collect()
                })
                .unwrap_or_default(),
            rows: obj
                .get("rows")
                .and_then(Value::as_array)
                .map(|rows| {
                    rows.iter()
                        .filter_map(Value::as_array)
                        .map(|row| row.iter().map(table_cell_text).collect())
                        .collect()
                })
                .unwrap_or_default(),
        }),
        "quote" => {
            let text = obj
                .get("entries")
                .and_then(Value::as_array)
                .map(|lines| {
                    lines
                        .iter()
                        .filter_map(Value::as_str)
                        .map(clean_tags)
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .unwrap_or_default();
            let by = obj.get("by").and_then(Value::as_str).map(clean_tags);
            Some(Entry::Text {
                text: match by {
                    Some(by) => format!("\u{201C}{text}\u{201D} \u{2014} {by}"),
                    None => format!("\u{201C}{text}\u{201D}"),
                },
            })
        }
        "abilityDc" => Some(Entry::Text {
            text: format!(
                "{} save DC = 8 + your proficiency bonus + your {} modifier",
                obj.get("name").and_then(Value::as_str).unwrap_or("Feature"),
                ability_names(obj)
            ),
        }),
        "abilityAttackMod" => Some(Entry::Text {
            text: format!(
                "{} attack modifier = your proficiency bonus + your {} modifier",
                obj.get("name").and_then(Value::as_str).unwrap_or("Feature"),
                ability_names(obj)
            ),
        }),
        // Cross-references to other features/optional features (e.g. the
        // "gain a subclass feature" placeholders): unresolvable as inline
        // text; subclass features render separately on the class page.
        _ => None,
    }
}

/// "item" entries carry their text under either `entry` (single) or `entries`.
fn item_entries(obj: &serde_json::Map<String, Value>) -> Vec<Entry> {
    if let Some(entry) = obj.get("entry") {
        return entry_from_value(entry).into_iter().collect();
    }
    entries_from_values(obj.get("entries").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[]))
}

fn ability_names(obj: &serde_json::Map<String, Value>) -> String {
    obj.get("attributes")
        .and_then(Value::as_array)
        .map(|attrs| {
            attrs
                .iter()
                .filter_map(Value::as_str)
                .map(crate::models::class::ability_label)
                .collect::<Vec<_>>()
                .join(" or ")
        })
        .unwrap_or_default()
}

/// Renders one progression-table cell to display text. Zero means "none yet"
/// in these columns (spell slots, infusions, ...), shown as an em dash.
pub(crate) fn progression_cell_text(cell: &Value) -> String {
    match cell {
        Value::Number(n) if n.as_u64() == Some(0) => "\u{2014}".to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(text) => clean_tags(text),
        Value::Object(obj) => match obj.get("type").and_then(Value::as_str) {
            Some("bonus") => match obj.get("value").and_then(Value::as_i64) {
                Some(value) => format!("+{value}"),
                None => "\u{2014}".to_string(),
            },
            Some("dice") => obj
                .get("toRoll")
                .and_then(Value::as_array)
                .map(|dice| {
                    dice.iter()
                        .filter_map(|d| {
                            let number = d.get("number").and_then(Value::as_u64)?;
                            let faces = d.get("faces").and_then(Value::as_u64)?;
                            Some(format!("{number}d{faces}"))
                        })
                        .collect::<Vec<_>>()
                        .join(" + ")
                })
                .unwrap_or_else(|| "\u{2014}".to_string()),
            _ => "\u{2014}".to_string(),
        },
        _ => "\u{2014}".to_string(),
    }
}

/// Cells inside feature tables (unlike the progression table, a literal 0 is
/// meaningful here, e.g. a d100 roll table bound).
fn table_cell_text(cell: &Value) -> String {
    match cell {
        Value::Number(n) => n.to_string(),
        other => progression_cell_text(other),
    }
}

/// Strips 5etools-style `{@tag body|meta}` markup down to display text:
/// "{@dice 1d12}" -> "1d12", "{@item rapier|phb}" -> "rapier". Resolves
/// innermost tags first so nested tags flatten correctly.
pub(crate) fn clean_tags(text: &str) -> String {
    let mut out = text.to_string();
    while let Some(start) = out.rfind("{@") {
        let Some(end) = out[start..].find('}').map(|rel| start + rel) else { break };
        let display = match out[start + 2..end].split_once(' ') {
            Some((_tag, body)) => body.split('|').next().unwrap_or("").to_string(),
            None => String::new(),
        };
        out.replace_range(start..=end, &display);
    }
    out
}

#[cfg(test)]
#[path = "entries_tests.rs"]
mod tests;
