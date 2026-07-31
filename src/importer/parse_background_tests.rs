use super::*;
use serde_json::json;

fn parse(records: Value) -> anyhow::Result<Vec<RawBackground>> {
    parse_background_file(&json!({ "background": records }).to_string())
}

fn base_acolyte() -> Value {
    json!({
        "name": "Acolyte",
        "source": "PHB",
        "skillProficiencies": [{ "insight": true, "religion": true }],
        "entries": [
            "intro text",
            { "type": "entries", "name": "Feature: Shelter", "entries": ["feature text"] }
        ]
    })
}

#[test]
fn plain_records_parse() {
    let parsed = parse(json!([base_acolyte()])).unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].name, "Acolyte");
    assert_eq!(parsed[0].skill_proficiencies.len(), 1);
    assert_eq!(parsed[0].entries.len(), 2);
}

#[test]
fn copy_inherits_base_fields_and_keeps_own_identity() {
    let parsed = parse(json!([
        base_acolyte(),
        {
            "name": "City Acolyte",
            "source": "XYZ",
            "_copy": { "name": "Acolyte", "source": "PHB" }
        }
    ]))
    .unwrap();
    let copy = &parsed[1];
    assert_eq!(copy.name, "City Acolyte");
    assert_eq!(copy.source, "XYZ");
    assert_eq!(copy.skill_proficiencies, parsed[0].skill_proficiencies);
    assert_eq!(copy.entries.len(), 2);
}

#[test]
fn copy_mods_can_insert_and_replace_entries() {
    let parsed = parse(json!([
        base_acolyte(),
        {
            "name": "Variant Acolyte",
            "_copy": {
                "name": "Acolyte",
                "_mod": {
                    "entries": [
                        { "mode": "insertArr", "index": 0, "items": "inserted first" },
                        {
                            "mode": "replaceArr",
                            "replace": "Feature: Shelter",
                            "items": { "type": "entries", "name": "Feature: New", "entries": ["new text"] }
                        }
                    ]
                }
            }
        }
    ]))
    .unwrap();
    let entries = &parsed[1].entries;
    assert_eq!(entries[0], json!("inserted first"));
    assert_eq!(entries[2].get("name"), Some(&json!("Feature: New")));
}

#[test]
fn replace_arr_accepts_an_index_address() {
    let parsed = parse(json!([
        base_acolyte(),
        {
            "name": "Indexed Acolyte",
            "_copy": {
                "name": "Acolyte",
                "_mod": {
                    "entries": { "mode": "replaceArr", "replace": { "index": 0 }, "items": "replaced" }
                }
            }
        }
    ]))
    .unwrap();
    assert_eq!(parsed[1].entries[0], json!("replaced"));
}

#[test]
fn chained_copies_resolve() {
    let parsed = parse(json!([
        base_acolyte(),
        { "name": "Copy One", "_copy": { "name": "Acolyte" } },
        { "name": "Copy Two", "_copy": { "name": "Copy One" } }
    ]))
    .unwrap();
    assert_eq!(parsed[2].name, "Copy Two");
    assert_eq!(parsed[2].entries.len(), 2);
}

#[test]
fn missing_copy_target_is_an_error() {
    let err = parse(json!([
        { "name": "Orphan", "_copy": { "name": "Nonexistent" } }
    ]))
    .unwrap_err();
    assert!(err.to_string().contains("Orphan"), "error should name the record: {err}");
}
