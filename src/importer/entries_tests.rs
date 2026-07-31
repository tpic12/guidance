use super::*;
use serde_json::json;

#[test]
fn clean_tags_strips_simple_and_piped_tags() {
    assert_eq!(clean_tags("roll {@dice 1d12} now"), "roll 1d12 now");
    assert_eq!(clean_tags("a {@item rapier|phb} blade"), "a rapier blade");
    assert_eq!(clean_tags("no tags here"), "no tags here");
}

#[test]
fn clean_tags_resolves_nested_tags() {
    assert_eq!(
        clean_tags("{@b {@item pouch|phb}} of gold"),
        "pouch of gold"
    );
}

#[test]
fn string_value_becomes_text_entry() {
    let entry = entry_from_value(&json!("plain {@skill Insight} text")).unwrap();
    assert_eq!(entry, Entry::Text { text: "plain Insight text".to_string() });
}

#[test]
fn list_and_item_values_become_nested_entries() {
    let entry = entry_from_value(&json!({
        "type": "list",
        "items": [
            "first",
            { "type": "item", "name": "Named:", "entry": "body text" }
        ]
    }))
    .unwrap();
    assert_eq!(
        entry,
        Entry::List {
            items: vec![
                Entry::Text { text: "first".to_string() },
                Entry::Item {
                    name: "Named:".to_string(),
                    entries: vec![Entry::Text { text: "body text".to_string() }],
                },
            ],
        }
    );
}

#[test]
fn entries_value_becomes_named_section() {
    let entry = entry_from_value(&json!({
        "type": "entries",
        "name": "Feature: Shelter",
        "entries": ["inner text"]
    }))
    .unwrap();
    assert_eq!(
        entry,
        Entry::Section {
            name: Some("Feature: Shelter".to_string()),
            entries: vec![Entry::Text { text: "inner text".to_string() }],
        }
    );
}

#[test]
fn table_value_keeps_labels_rows_and_meaningful_zeros() {
    let entry = entry_from_value(&json!({
        "type": "table",
        "caption": "Flaws",
        "colLabels": ["d8", "Flaw"],
        "rows": [[0, "Your mark hisses."]]
    }))
    .unwrap();
    assert_eq!(
        entry,
        Entry::Table {
            caption: Some("Flaws".to_string()),
            col_labels: vec!["d8".to_string(), "Flaw".to_string()],
            rows: vec![vec!["0".to_string(), "Your mark hisses.".to_string()]],
        }
    );
}

#[test]
fn unknown_and_unresolvable_types_are_dropped() {
    assert_eq!(entry_from_value(&json!({ "type": "refClassFeature" })), None);
    assert_eq!(entry_from_value(&json!(42)), None);
}

#[test]
fn progression_cells_render_zero_as_dash_and_format_dice() {
    assert_eq!(progression_cell_text(&json!(0)), "\u{2014}");
    assert_eq!(progression_cell_text(&json!(3)), "3");
    assert_eq!(
        progression_cell_text(&json!({ "type": "bonus", "value": 2 })),
        "+2"
    );
    assert_eq!(
        progression_cell_text(
            &json!({ "type": "dice", "toRoll": [{ "number": 1, "faces": 6 }] })
        ),
        "1d6"
    );
}
