use super::*;
use crate::importer::parse_class_spells::parse_class_spell_lookup;

#[test]
fn flattens_grant_source_and_class_name_into_links() {
    let raw = r#"{
        "phb": {
            "fireball": {
                "class": {"PHB": {"Sorcerer": true, "Wizard": true}}
            }
        }
    }"#;
    let lookup = parse_class_spell_lookup(raw).unwrap();
    let mut links = class_spell_links_from_parsed(&lookup);
    links.sort_by(|a, b| a.class_name.cmp(&b.class_name));

    assert_eq!(links.len(), 2);
    assert_eq!(links[0].spell_name_lower, "fireball");
    assert_eq!(links[0].spell_source, "phb");
    assert_eq!(links[0].class_name, "Sorcerer");
    assert_eq!(links[1].class_name, "Wizard");
}

#[test]
fn same_class_under_multiple_grant_sources_yields_multiple_links() {
    // Resolved separately at seed time via INSERT OR IGNORE, so duplicates
    // here (one per grant source) are expected, not deduped in transform.
    let raw = r#"{
        "phb": {
            "guidance": {
                "class": {"PHB": {"Cleric": true}, "XGE": {"Cleric": true}}
            }
        }
    }"#;
    let lookup = parse_class_spell_lookup(raw).unwrap();
    let links = class_spell_links_from_parsed(&lookup);
    assert_eq!(links.len(), 2);
    assert!(links.iter().all(|link| link.class_name == "Cleric"));
}

#[test]
fn empty_class_map_yields_no_links() {
    let raw = r#"{"phb": {"prestidigitation": {}}}"#;
    let lookup = parse_class_spell_lookup(raw).unwrap();
    assert!(class_spell_links_from_parsed(&lookup).is_empty());
}

#[test]
fn flattens_subclass_grants_into_links() {
    let raw = r#"{
        "phb": {
            "shield": {
                "subclass": {
                    "PHB": {
                        "Fighter": {"PHB": {"Eldritch Knight": {"name": "Eldritch Knight"}}},
                        "Rogue": {"PHB": {"Arcane Trickster": {"name": "Arcane Trickster"}}}
                    }
                }
            }
        }
    }"#;
    let lookup = parse_class_spell_lookup(raw).unwrap();
    let mut links = subclass_spell_links_from_parsed(&lookup);
    links.sort_by(|a, b| a.class_name.cmp(&b.class_name));

    assert_eq!(links.len(), 2);
    assert_eq!(links[0].spell_name_lower, "shield");
    assert_eq!(links[0].spell_source, "phb");
    assert_eq!(links[0].class_name, "Fighter");
    assert_eq!(links[0].subclass_short_name, "Eldritch Knight");
    assert_eq!(links[0].subclass_source, "PHB");
    assert_eq!(links[1].class_name, "Rogue");
    assert_eq!(links[1].subclass_short_name, "Arcane Trickster");
}

#[test]
fn resolves_subclass_source_independently_of_class_source() {
    // Real-world shape: class source (PHB, Warlock's book) differs from
    // subclass source (TCE, Fathomless's book) — see gendata-spell-source-lookup.json.
    let raw = r#"{
        "phb": {
            "thunderwave": {
                "subclass": {
                    "PHB": {
                        "Warlock": {"TCE": {"Fathomless": {"name": "The Fathomless"}}}
                    }
                }
            }
        }
    }"#;
    let lookup = parse_class_spell_lookup(raw).unwrap();
    let links = subclass_spell_links_from_parsed(&lookup);
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].class_name, "Warlock");
    assert_eq!(links[0].subclass_short_name, "Fathomless");
    assert_eq!(links[0].subclass_source, "TCE");
}

#[test]
fn empty_subclass_map_yields_no_links() {
    let raw = r#"{"phb": {"prestidigitation": {}}}"#;
    let lookup = parse_class_spell_lookup(raw).unwrap();
    assert!(subclass_spell_links_from_parsed(&lookup).is_empty());
}
