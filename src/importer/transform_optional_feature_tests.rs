use super::*;
use crate::importer::parse_optional_feature::parse_optional_feature_file;
use serde_json::json;

fn feature_from(json_value: serde_json::Value) -> OptionalFeature {
    let raw = format!(r#"{{"optionalfeature": [{json_value}]}}"#);
    let parsed = parse_optional_feature_file(&raw).unwrap();
    optional_features_from_parsed(parsed).unwrap().remove(0)
}

#[test]
fn basic_fields_and_id_slugify() {
    let feature = feature_from(json!({
        "name": "Agonizing Blast",
        "source": "PHB",
        "featureType": ["EI"],
        "entries": ["When you cast eldritch blast..."]
    }));
    assert_eq!(feature.id, "agonizing-blast");
    assert_eq!(feature.canonical_id, "Agonizing Blast|PHB");
    assert_eq!(feature.name, "Agonizing Blast");
    assert_eq!(feature.source, "PHB");
    assert_eq!(feature.feature_types, vec![FeatureType::EldritchInvocation]);
    assert!(feature.prerequisites.is_empty());
    assert!(feature.consumes.is_none());
}

#[test]
fn multiple_feature_types_are_all_kept() {
    let feature = feature_from(json!({
        "name": "Archery",
        "source": "PHB",
        "featureType": ["FS:F", "FS:R"],
        "entries": []
    }));
    assert_eq!(
        feature.feature_types,
        vec![FeatureType::FightingStyleFighter, FeatureType::FightingStyleRanger]
    );
}

#[test]
fn unrecognized_feature_type_codes_are_tolerated_and_dropped() {
    let feature = feature_from(json!({
        "name": "Something New",
        "source": "UA",
        "featureType": ["EI", "ZZZ"],
        "entries": []
    }));
    assert_eq!(feature.feature_types, vec![FeatureType::EldritchInvocation]);
}

#[test]
fn level_class_and_subclass_prerequisite_is_captured_structurally() {
    let feature = feature_from(json!({
        "name": "Breath of Winter",
        "source": "PHB",
        "featureType": ["ED"],
        "prerequisite": [{
            "level": {
                "level": 17,
                "class": {"name": "Monk"},
                "subclass": {"name": "Four Elements"}
            }
        }],
        "entries": []
    }));
    assert_eq!(feature.prerequisites.len(), 1);
    let prereq = &feature.prerequisites[0];
    assert_eq!(prereq.level, Some(17));
    assert_eq!(prereq.class_name.as_deref(), Some("Monk"));
    assert_eq!(prereq.class_source, None);
    assert_eq!(prereq.subclass_name.as_deref(), Some("Four Elements"));
    assert_eq!(prereq.subclass_source, None);
}

#[test]
fn pact_and_level_combine_within_one_prerequisite_option() {
    let feature = feature_from(json!({
        "name": "Thirsting Blade",
        "source": "PHB",
        "featureType": ["EI"],
        "prerequisite": [{
            "pact": "Blade",
            "level": {"level": 5, "class": {"name": "Warlock"}}
        }],
        "entries": []
    }));
    let prereq = &feature.prerequisites[0];
    assert_eq!(prereq.pact.as_deref(), Some("Blade"));
    assert_eq!(prereq.level, Some(5));
    assert_eq!(prereq.class_name.as_deref(), Some("Warlock"));
}

#[test]
fn required_spell_names_strip_hash_and_pipe_suffixes() {
    let feature = feature_from(json!({
        "name": "Agonizing Blast",
        "source": "PHB",
        "featureType": ["EI"],
        "prerequisite": [{"spell": ["eldritch blast#c"]}],
        "entries": []
    }));
    assert_eq!(feature.prerequisites[0].spells, vec!["eldritch blast".to_string()]);
}

#[test]
fn item_prerequisite_becomes_freeform_other_text() {
    let feature = feature_from(json!({
        "name": "Arcane Propulsion Armor",
        "source": "TCE",
        "featureType": ["AI"],
        "prerequisite": [{
            "level": {"level": 14, "class": {"name": "Artificer", "source": "TCE"}},
            "item": ["A suit of armor (requires attunement)"]
        }],
        "entries": []
    }));
    let prereq = &feature.prerequisites[0];
    assert_eq!(prereq.class_source.as_deref(), Some("TCE"));
    assert_eq!(prereq.other.as_deref(), Some("A suit of armor (requires attunement)"));
}

#[test]
fn multiple_prerequisite_options_are_all_kept_as_separate_or_alternatives() {
    let feature = feature_from(json!({
        "name": "Gift of the Ever-Living Ones",
        "source": "PHB",
        "featureType": ["EI"],
        "prerequisite": [{"pact": "Chain"}],
        "entries": []
    }));
    assert_eq!(feature.prerequisites.len(), 1);

    let feature = feature_from(json!({
        "name": "Bond of the Talisman",
        "source": "TCE",
        "featureType": ["EI"],
        "prerequisite": [
            {"pact": "Talisman"},
            {"level": {"level": 12, "class": {"name": "Warlock"}}}
        ],
        "entries": []
    }));
    assert_eq!(feature.prerequisites.len(), 2);
}

#[test]
fn consumes_shapes_parse_amount_min_and_max() {
    let feature = feature_from(json!({
        "name": "Empowered Spell",
        "source": "PHB",
        "featureType": ["MM"],
        "consumes": {"name": "Sorcery Point", "amountMin": 1, "amountMax": 9},
        "entries": []
    }));
    let consumes = feature.consumes.unwrap();
    assert_eq!(consumes.name, "Sorcery Point");
    assert_eq!(consumes.amount, None);
    assert_eq!(consumes.amount_min, Some(1));
    assert_eq!(consumes.amount_max, Some(9));
}

#[test]
fn consumes_with_no_amount_is_still_captured() {
    let feature = feature_from(json!({
        "name": "Sculptor of Flesh",
        "source": "PHB",
        "featureType": ["EI"],
        "consumes": {"name": "Arcane Shot"},
        "entries": []
    }));
    let consumes = feature.consumes.unwrap();
    assert_eq!(consumes.name, "Arcane Shot");
    assert_eq!(consumes.amount, None);
}
