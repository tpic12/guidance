use super::*;

#[test]
fn parses_nested_class_grant_shape() {
    let raw = r#"{
        "phb": {
            "fireball": {
                "class": {"PHB": {"Sorcerer": true, "Wizard": true}}
            }
        }
    }"#;
    let lookup = parse_class_spell_lookup(raw).unwrap();
    let entry = &lookup["phb"]["fireball"];
    assert_eq!(entry.class["PHB"].len(), 2);
    assert!(entry.class["PHB"].contains_key("Sorcerer"));
    assert!(entry.class["PHB"].contains_key("Wizard"));
}

#[test]
fn missing_class_key_defaults_to_empty() {
    let raw = r#"{"phb": {"prestidigitation": {}}}"#;
    let lookup = parse_class_spell_lookup(raw).unwrap();
    assert!(lookup["phb"]["prestidigitation"].class.is_empty());
}

#[test]
fn parses_nested_subclass_grant_shape_and_tolerates_extra_unmodeled_keys_like_race() {
    let raw = r#"{
        "phb": {
            "acid splash": {
                "class": {"PHB": {"Sorcerer": true}},
                "subclass": {"PHB": {"Cleric": {"SCAG": {"Arcana": {"name": "Arcana Domain"}}}}},
                "race": {"PHB": {"Elf (High)": {}}}
            }
        }
    }"#;
    let lookup = parse_class_spell_lookup(raw).unwrap();
    let entry = &lookup["phb"]["acid splash"];
    assert!(entry.class["PHB"].contains_key("Sorcerer"));
    assert!(entry.subclass["PHB"]["Cleric"]["SCAG"].contains_key("Arcana"));
}

#[test]
fn missing_subclass_key_defaults_to_empty() {
    let raw = r#"{"phb": {"prestidigitation": {}}}"#;
    let lookup = parse_class_spell_lookup(raw).unwrap();
    assert!(lookup["phb"]["prestidigitation"].subclass.is_empty());
}
