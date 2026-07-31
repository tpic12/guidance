use super::*;
use serde_json::json;

#[test]
fn copy_merges_parent_fields_child_wins_on_conflict() {
    let parent = json!({"name": "Fake Base", "source": "TBK", "rarity": "common", "weight": 2});
    let child = json!({
        "name": "Fake Child", "source": "TBK", "rarity": "rare",
        "_copy": {"name": "Fake Base", "source": "TBK"}
    });

    let resolved = resolve_item_copies(vec![parent, child]);
    let resolved_child = resolved.iter().find(|v| v["name"] == "Fake Child").unwrap();

    assert_eq!(resolved_child["rarity"], "rare");
    assert_eq!(resolved_child["weight"], 2);
}

#[test]
fn insert_arr_splices_at_the_given_index() {
    let parent = json!({"name": "Fake Base", "source": "TBK", "entries": ["first", "second"]});
    let child = json!({
        "name": "Fake Child", "source": "TBK",
        "_copy": {
            "name": "Fake Base", "source": "TBK",
            "_mod": {"entries": {"mode": "insertArr", "index": 1, "items": "middle"}}
        }
    });

    let resolved = resolve_item_copies(vec![parent, child]);
    let resolved_child = resolved.iter().find(|v| v["name"] == "Fake Child").unwrap();

    assert_eq!(resolved_child["entries"], json!(["first", "middle", "second"]));
}

#[test]
fn insert_arr_with_negative_index_appends() {
    let parent = json!({"name": "Fake Base", "source": "TBK", "entries": ["first"]});
    let child = json!({
        "name": "Fake Child", "source": "TBK",
        "_copy": {
            "name": "Fake Base", "source": "TBK",
            "_mod": {"entries": {"mode": "insertArr", "index": -1, "items": "last"}}
        }
    });

    let resolved = resolve_item_copies(vec![parent, child]);
    let resolved_child = resolved.iter().find(|v| v["name"] == "Fake Child").unwrap();

    assert_eq!(resolved_child["entries"], json!(["first", "last"]));
}

#[test]
fn append_arr_appends_like_insert_arr_negative_index() {
    let parent = json!({"name": "Fake Base", "source": "TBK", "entries": ["first"]});
    let child = json!({
        "name": "Fake Child", "source": "TBK",
        "_copy": {
            "name": "Fake Base", "source": "TBK",
            "_mod": {"entries": {"mode": "appendArr", "items": "last"}}
        }
    });

    let resolved = resolve_item_copies(vec![parent, child]);
    let resolved_child = resolved.iter().find(|v| v["name"] == "Fake Child").unwrap();

    assert_eq!(resolved_child["entries"], json!(["first", "last"]));
}

#[test]
fn unsupported_mod_mode_leaves_entries_untouched() {
    let parent = json!({"name": "Fake Base", "source": "TBK", "entries": ["first"]});
    let child = json!({
        "name": "Fake Child", "source": "TBK",
        "_copy": {
            "name": "Fake Base", "source": "TBK",
            "_mod": {"entries": {"mode": "replaceArr", "items": "replacement"}}
        }
    });

    let resolved = resolve_item_copies(vec![parent, child]);
    let resolved_child = resolved.iter().find(|v| v["name"] == "Fake Child").unwrap();

    assert_eq!(resolved_child["entries"], json!(["first"]));
}

#[test]
fn copy_chains_resolve_recursively() {
    let grandparent = json!({"name": "Fake Grandparent", "source": "TBK", "rarity": "common"});
    let parent = json!({
        "name": "Fake Parent", "source": "TBK",
        "_copy": {"name": "Fake Grandparent", "source": "TBK"}
    });
    let child = json!({
        "name": "Fake Child", "source": "TBK",
        "_copy": {"name": "Fake Parent", "source": "TBK"}
    });

    let resolved = resolve_item_copies(vec![grandparent, parent, child]);
    let resolved_child = resolved.iter().find(|v| v["name"] == "Fake Child").unwrap();

    assert_eq!(resolved_child["rarity"], "common");
}

#[test]
fn unique_id_disambiguates_collisions() {
    let mut used = HashSet::new();
    assert_eq!(unique_id("fake-item".to_string(), &mut used), "fake-item");
    assert_eq!(unique_id("fake-item".to_string(), &mut used), "fake-item-2");
    assert_eq!(unique_id("fake-item".to_string(), &mut used), "fake-item-3");
}

#[test]
fn property_label_reads_first_entry_name() {
    let properties = vec![RawItemProperty {
        abbreviation: "H".to_string(),
        entries: vec![json!({"type": "entries", "name": "Heavy", "entries": ["text"]})],
    }];

    let labels = property_label_map(&properties);

    assert_eq!(labels.get("H"), Some(&"Heavy".to_string()));
}

#[test]
fn property_label_falls_back_to_abbreviation_when_missing() {
    let properties = vec![RawItemProperty { abbreviation: "Z".to_string(), entries: vec![] }];

    let labels = property_label_map(&properties);

    assert_eq!(labels.get("Z"), Some(&"Z".to_string()));
}

#[test]
fn item_type_code_strips_source_suffix_before_lookup() {
    let types = vec![RawItemType { abbreviation: "M".to_string(), name: "Melee Weapon".to_string() }];
    let type_labels = type_label_map(&types);
    let raw = RawItem {
        name: "Fake Sword".to_string(),
        source: "TBK".to_string(),
        item_type: Some("M|DMG".to_string()),
        ..Default::default()
    };
    let mut used = HashSet::new();

    let item = item_from_raw(raw, false, &HashMap::new(), &type_labels, &HashMap::new(), &mut used);

    assert_eq!(item.item_type_code, Some("M".to_string()));
    assert_eq!(item.item_type_label, Some("Melee Weapon".to_string()));
}

#[test]
fn wondrous_item_with_no_type_gets_synthesized_label() {
    let raw =
        RawItem { name: "Fake Wondrous".to_string(), source: "TBK".to_string(), wondrous: true, ..Default::default() };
    let mut used = HashSet::new();

    let item = item_from_raw(raw, false, &HashMap::new(), &HashMap::new(), &HashMap::new(), &mut used);

    assert_eq!(item.item_type_label, Some("Wondrous Item".to_string()));
}

#[test]
fn base_item_name_resolves_case_insensitively() {
    let mut base_names = HashMap::new();
    base_names.insert(("sickle".to_string(), "phb".to_string()), "Sickle".to_string());
    let raw = RawItem {
        name: "Fake +1 Sickle".to_string(),
        source: "TBK".to_string(),
        base_item: Some("Sickle|PHB".to_string()),
        ..Default::default()
    };
    let mut used = HashSet::new();

    let item = item_from_raw(raw, false, &HashMap::new(), &HashMap::new(), &base_names, &mut used);

    assert_eq!(item.base_item_name, Some("Sickle".to_string()));
}

#[test]
fn req_attune_string_sets_note_and_requires_true() {
    let raw = RawItem {
        name: "Fake Ring".to_string(),
        source: "TBK".to_string(),
        req_attune: Some(json!("by a fake wizard")),
        ..Default::default()
    };
    let mut used = HashSet::new();

    let item = item_from_raw(raw, false, &HashMap::new(), &HashMap::new(), &HashMap::new(), &mut used);

    assert!(item.requires_attunement);
    assert_eq!(item.attunement_note, Some("by a fake wizard".to_string()));
}

#[test]
fn magic_bonus_strings_parse_to_signed_integers() {
    assert_eq!(parse_bonus("+1"), Some(1));
    assert_eq!(parse_bonus("+2"), Some(2));
    assert_eq!(parse_bonus(""), None);
}

#[test]
fn rarity_rank_orders_by_severity_not_alphabetically() {
    assert!(rarity_rank("common") < rarity_rank("uncommon"));
    assert!(rarity_rank("uncommon") < rarity_rank("legendary"));
    assert!(rarity_rank("legendary") < rarity_rank("artifact"));
    assert!(rarity_rank("artifact") < rarity_rank("varies"));
}
