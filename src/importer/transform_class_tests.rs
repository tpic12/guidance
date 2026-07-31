use super::*;
use crate::importer::parse_class::parse_class_file;
use crate::models::optional_feature::FeatureType;
use crate::models::skill::SkillGrant;
use serde_json::json;

#[test]
fn dense_progression_passes_dense_arrays_through() {
    let shape = RawOptionalFeatureProgressionShape::Dense(vec![0, 2, 2, 3]);
    assert_eq!(dense_progression(&shape), vec![0, 2, 2, 3]);
}

#[test]
fn dense_progression_forward_fills_sparse_maps_from_unordered_keys() {
    // HashMap iteration order isn't level order, so this exercises the sort
    // step: keys inserted out of order must still forward-fill correctly.
    let mut map = std::collections::HashMap::new();
    map.insert("10".to_string(), 3u8);
    map.insert("3".to_string(), 2u8);
    let shape = RawOptionalFeatureProgressionShape::Sparse(map);
    let dense = dense_progression(&shape);
    assert_eq!(dense.len(), 20);
    assert_eq!(dense[0], 0); // level 1: before first breakpoint
    assert_eq!(dense[1], 0); // level 2: still before the level-3 breakpoint
    assert_eq!(dense[2], 2); // level 3: breakpoint hits
    assert_eq!(dense[8], 2); // level 9: holds
    assert_eq!(dense[9], 3); // level 10: next breakpoint
    assert_eq!(dense[19], 3); // level 20: holds to the end
}

#[test]
fn optional_feature_progressions_from_raw_maps_codes_and_drops_unknown() {
    let raw = vec![
        RawOptionalFeatureProgressionEntry {
            feature_type: vec!["EI".to_string()],
            progression: RawOptionalFeatureProgressionShape::Dense(vec![0, 2, 2]),
        },
        RawOptionalFeatureProgressionEntry {
            feature_type: vec!["ZZZ".to_string()],
            progression: RawOptionalFeatureProgressionShape::Dense(vec![0, 1, 1]),
        },
    ];
    let progressions = optional_feature_progressions_from_raw(&raw);
    assert_eq!(progressions.len(), 1);
    assert_eq!(progressions[0].feature_type, FeatureType::EldritchInvocation);
    assert_eq!(progressions[0].known, vec![0, 2, 2]);
}

fn warlock_shaped_file(
    class_progression: serde_json::Value,
    subclass_progression: serde_json::Value,
) -> String {
    format!(
        r#"{{
        "class": [{{
            "name": "Fake Warlock",
            "source": "PHB",
            "hd": {{"number": 1, "faces": 8}},
            "proficiency": ["wis", "cha"],
            "optionalfeatureProgression": {class_progression}
        }}],
        "subclass": [{{
            "name": "The Fake Patron",
            "shortName": "Fake Patron",
            "source": "PHB",
            "className": "Fake Warlock",
            "classSource": "PHB",
            "optionalfeatureProgression": {subclass_progression}
        }}],
        "classFeature": [],
        "subclassFeature": []
    }}"#
    )
}

#[test]
fn class_and_subclass_progressions_of_the_same_type_are_kept_separate_for_later_summing() {
    let raw = warlock_shaped_file(
        json!([{"featureType": ["EI"], "progression": [0,2,2,3]}]),
        json!([{"featureType": ["EI"], "progression": {"10": 1}}]),
    );

    let bundles = class_bundles_from_parsed(parse_class_file(&raw).unwrap()).unwrap();
    let class = &bundles[0].class;
    assert_eq!(class.optional_feature_progressions.len(), 1);
    assert_eq!(class.optional_feature_progressions[0].feature_type, FeatureType::EldritchInvocation);
    assert_eq!(class.optional_feature_progressions[0].known, vec![0, 2, 2, 3]);

    let (subclass, _features, _grants) = &bundles[0].subclasses[0];
    assert_eq!(subclass.optional_feature_progressions.len(), 1);
    assert_eq!(subclass.optional_feature_progressions[0].feature_type, FeatureType::EldritchInvocation);
    // Sparse {"10": 1}, forward-filled: 0 until level 10, then 1.
    assert_eq!(subclass.optional_feature_progressions[0].known[8], 0); // level 9
    assert_eq!(subclass.optional_feature_progressions[0].known[9], 1); // level 10
}

#[test]
fn multiple_feature_types_on_one_class_stay_independent() {
    let raw = warlock_shaped_file(
        json!([
            {"featureType": ["EI"], "progression": [0,2,2,3]},
            {"featureType": ["PB"], "progression": {"3": 1}}
        ]),
        json!([]),
    );

    let bundles = class_bundles_from_parsed(parse_class_file(&raw).unwrap()).unwrap();
    let progressions = &bundles[0].class.optional_feature_progressions;
    assert_eq!(progressions.len(), 2);
    assert!(progressions.iter().any(|p| p.feature_type == FeatureType::EldritchInvocation));
    assert!(progressions.iter().any(|p| p.feature_type == FeatureType::PactBoon));
}

#[test]
fn fixed_skill_names_become_a_single_fixed_grant() {
    let values = vec![json!("athletics"), json!("stealth")];
    let grants = skill_grants_from_class(&values);
    assert_eq!(
        grants,
        vec![SkillGrant::Fixed { skills: vec!["athletics".to_string(), "stealth".to_string()] }]
    );
}

#[test]
fn choose_block_matches_the_fake_warrior_fixture_shape() {
    let values = vec![json!({"choose": {"from": ["athletics", "intimidation", "survival"], "count": 2}})];
    let grants = skill_grants_from_class(&values);
    assert_eq!(
        grants,
        vec![SkillGrant::Choose {
            count: 2,
            from: vec!["athletics".to_string(), "intimidation".to_string(), "survival".to_string()],
        }]
    );
}

#[test]
fn any_block_becomes_an_any_grant() {
    let values = vec![json!({"any": 3})];
    assert_eq!(skill_grants_from_class(&values), vec![SkillGrant::Any { count: 3 }]);
}

#[test]
fn fixed_and_choose_can_combine_with_fixed_listed_first() {
    let values = vec![
        json!({"choose": {"from": ["arcana", "history"], "count": 1}}),
        json!("religion"),
    ];
    let grants = skill_grants_from_class(&values);
    assert_eq!(
        grants,
        vec![
            SkillGrant::Fixed { skills: vec!["religion".to_string()] },
            SkillGrant::Choose { count: 1, from: vec!["arcana".to_string(), "history".to_string()] },
        ]
    );
}

#[test]
fn armor_and_weapon_labels_use_friendly_names() {
    assert_eq!(armor_label("light"), "Light armor");
    assert_eq!(armor_label("shield"), "Shields");
    assert_eq!(weapon_label("simple"), "Simple weapons");
    assert_eq!(weapon_label("martial"), "Martial weapons");
}

#[test]
fn caster_progression_normalizes_5etools_fraction_codes() {
    assert_eq!(normalize_caster_progression(Some("full".to_string())), Some("full".to_string()));
    assert_eq!(normalize_caster_progression(Some("1/2".to_string())), Some("half".to_string()));
    assert_eq!(normalize_caster_progression(Some("1/3".to_string())), Some("third".to_string()));
    assert_eq!(normalize_caster_progression(Some("pact".to_string())), Some("pact".to_string()));
    assert_eq!(normalize_caster_progression(None), None);
}

#[test]
fn subclass_granted_spellcasting_fields_flow_through_and_normalize() {
    // Mirrors the real Eldritch Knight shape in fixtures/classes/fighter.json:
    // a non-caster base class whose subclass carries its own spellcasting.
    let raw = r#"{
        "class": [{
            "name": "Fighter",
            "source": "PHB",
            "hd": {"number": 1, "faces": 10},
            "proficiency": ["str", "con"]
        }],
        "subclass": [{
            "name": "Eldritch Knight",
            "shortName": "Eldritch Knight",
            "source": "PHB",
            "className": "Fighter",
            "classSource": "PHB",
            "casterProgression": "1/3",
            "spellcastingAbility": "int",
            "spellsKnownProgression": [0, 0, 3, 4],
            "cantripProgression": [0, 0, 2, 2]
        }],
        "classFeature": [],
        "subclassFeature": []
    }"#;

    let file = parse_class_file(raw).unwrap();
    let bundles = class_bundles_from_parsed(file).unwrap();
    assert_eq!(bundles.len(), 1);
    let (subclass, _features, _grants) = &bundles[0].subclasses[0];

    assert_eq!(subclass.caster_progression, Some("third".to_string()));
    assert_eq!(subclass.spellcasting_ability, Some("int".to_string()));
    assert_eq!(subclass.spells_known_progression, Some(vec![0, 0, 3, 4]));
    assert_eq!(subclass.cantrips_known_progression, Some(vec![0, 0, 2, 2]));
    // The base class itself stays a correct non-caster.
    assert_eq!(bundles[0].class.caster_progression, None);
}

#[test]
fn multiclassing_proficiencies_gained_populate_multiclass_proficiencies() {
    let raw = r#"{
        "class": [{
            "name": "Fighter",
            "source": "PHB",
            "hd": {"number": 1, "faces": 10},
            "proficiency": ["str", "con"],
            "startingProficiencies": {
                "armor": ["light", "medium", "shield"],
                "weapons": ["simple", "martial"],
                "skills": [{"choose": {"from": ["acrobatics", "athletics"], "count": 2}}]
            },
            "multiclassing": {
                "proficienciesGained": {
                    "armor": ["light"],
                    "skills": [{"choose": {"from": ["acrobatics", "athletics", "insight"], "count": 1}}]
                }
            }
        }],
        "subclass": [],
        "classFeature": [],
        "subclassFeature": []
    }"#;

    let file = parse_class_file(raw).unwrap();
    let bundles = class_bundles_from_parsed(file).unwrap();
    let class = &bundles[0].class;

    assert_eq!(class.proficiencies.armor, vec!["Light armor", "Medium armor", "Shields"]);
    assert_eq!(
        class.multiclass_proficiencies.armor,
        vec!["Light armor".to_string()],
        "multiclass proficiencies are a smaller, separate set from the class's own starting proficiencies"
    );
    assert_eq!(
        class.multiclass_proficiencies.skills,
        vec![SkillGrant::Choose {
            count: 1,
            from: vec!["acrobatics".to_string(), "athletics".to_string(), "insight".to_string()],
        }]
    );
}

#[test]
fn a_class_with_no_multiclassing_block_gets_empty_multiclass_proficiencies() {
    let raw = r#"{
        "class": [{
            "name": "Fighter",
            "source": "PHB",
            "hd": {"number": 1, "faces": 10},
            "proficiency": ["str", "con"]
        }],
        "subclass": [],
        "classFeature": [],
        "subclassFeature": []
    }"#;

    let file = parse_class_file(raw).unwrap();
    let bundles = class_bundles_from_parsed(file).unwrap();
    assert_eq!(bundles[0].class.multiclass_proficiencies, crate::models::class::Proficiencies::default());
}

fn class_with_requirements(requirements: serde_json::Value) -> String {
    format!(
        r#"{{
        "class": [{{
            "name": "Fake Class",
            "source": "PHB",
            "hd": {{"number": 1, "faces": 8}},
            "proficiency": ["str"],
            "multiclassing": {{
                "requirements": {requirements}
            }}
        }}],
        "subclass": [],
        "classFeature": [],
        "subclassFeature": []
    }}"#
    )
}

#[test]
fn a_flat_single_key_requirement_becomes_one_and_group() {
    let raw = class_with_requirements(json!({"str": 13}));
    let bundles = class_bundles_from_parsed(parse_class_file(&raw).unwrap()).unwrap();
    assert_eq!(bundles[0].class.multiclass_ability_prerequisites, vec![vec![("str".to_string(), 13)]]);
}

#[test]
fn a_flat_multi_key_requirement_becomes_one_and_group_with_both_entries() {
    let raw = class_with_requirements(json!({"dex": 13, "wis": 13}));
    let bundles = class_bundles_from_parsed(parse_class_file(&raw).unwrap()).unwrap();
    let prereqs = &bundles[0].class.multiclass_ability_prerequisites;
    assert_eq!(prereqs.len(), 1, "both abilities are required together as a single AND-group");
    let mut group = prereqs[0].clone();
    group.sort();
    assert_eq!(group, vec![("dex".to_string(), 13), ("wis".to_string(), 13)]);
}

#[test]
fn a_lone_or_element_with_multiple_keys_becomes_alternative_or_groups() {
    // Str 13 or Dex 13, not both.
    let raw = class_with_requirements(json!({"or": [{"str": 13, "dex": 13}]}));
    let bundles = class_bundles_from_parsed(parse_class_file(&raw).unwrap()).unwrap();
    let mut prereqs = bundles[0].class.multiclass_ability_prerequisites.clone();
    prereqs.sort();
    assert_eq!(prereqs, vec![vec![("dex".to_string(), 13)], vec![("str".to_string(), 13)]]);
}

#[test]
fn multiple_or_elements_become_alternative_and_groups() {
    let raw = class_with_requirements(json!({"or": [{"str": 13, "cha": 13}, {"dex": 13, "wis": 13}]}));
    let bundles = class_bundles_from_parsed(parse_class_file(&raw).unwrap()).unwrap();
    let prereqs = &bundles[0].class.multiclass_ability_prerequisites;
    assert_eq!(prereqs.len(), 2);
    for group in prereqs {
        assert_eq!(group.len(), 2, "each alternative keeps both of its own requirements as an AND-group");
    }
}

#[test]
fn a_class_with_no_requirements_gets_empty_prerequisites() {
    let raw = r#"{
        "class": [{
            "name": "Fake Class",
            "source": "PHB",
            "hd": {"number": 1, "faces": 8},
            "proficiency": ["str"]
        }],
        "subclass": [],
        "classFeature": [],
        "subclassFeature": []
    }"#;
    let bundles = class_bundles_from_parsed(parse_class_file(raw).unwrap()).unwrap();
    assert!(bundles[0].class.multiclass_ability_prerequisites.is_empty());
}

fn cleric_shaped_file(additional_spells: serde_json::Value) -> String {
    format!(
        r#"{{
        "class": [{{
            "name": "Cleric",
            "source": "PHB",
            "hd": {{"number": 1, "faces": 8}},
            "proficiency": ["wis", "cha"],
            "casterProgression": "full",
            "spellcastingAbility": "wis"
        }}],
        "subclass": [{{
            "name": "Life Domain",
            "shortName": "Life",
            "source": "PHB",
            "className": "Cleric",
            "classSource": "PHB",
            "additionalSpells": {additional_spells}
        }}],
        "classFeature": [],
        "subclassFeature": []
    }}"#
    )
}

#[test]
fn additional_spells_prepared_and_known_become_granted_refs() {
    let raw = cleric_shaped_file(json!([{
        "prepared": {"1": ["bless", "cure wounds"], "3": ["lesser restoration"]},
        "known": {"1": ["light#c"]}
    }]));

    let bundles = class_bundles_from_parsed(parse_class_file(&raw).unwrap()).unwrap();
    let (subclass, _features, grants) = &bundles[0].subclasses[0];

    // Common (ungrouped) shape: no fan-out, exactly one subclass produced.
    assert_eq!(bundles[0].subclasses.len(), 1);
    assert_eq!(subclass.name, "Life Domain");

    let mut sorted = grants.clone();
    sorted.sort_by(|a, b| (a.level, &a.spell_name_lower).cmp(&(b.level, &b.spell_name_lower)));
    let mut expected = vec![
        GrantedSpellRef { spell_name_lower: "light".to_string(), level: 1 },
        GrantedSpellRef { spell_name_lower: "bless".to_string(), level: 1 },
        GrantedSpellRef { spell_name_lower: "cure wounds".to_string(), level: 1 },
        GrantedSpellRef { spell_name_lower: "lesser restoration".to_string(), level: 3 },
    ];
    expected.sort_by(|a, b| (a.level, &a.spell_name_lower).cmp(&(b.level, &b.spell_name_lower)));
    assert_eq!(sorted, expected);
}

#[test]
fn additional_spells_strips_hash_and_pipe_suffixes() {
    let raw = cleric_shaped_file(json!([{
        "prepared": {"7": ["fire shield|", "guardian of faith"]},
        "known": {"1": ["spare the dying#c"]}
    }]));

    let bundles = class_bundles_from_parsed(parse_class_file(&raw).unwrap()).unwrap();
    let (_subclass, _features, grants) = &bundles[0].subclasses[0];
    let names: std::collections::HashSet<&str> =
        grants.iter().map(|g| g.spell_name_lower.as_str()).collect();
    assert!(names.contains("fire shield"));
    assert!(names.contains("guardian of faith"));
    assert!(names.contains("spare the dying"));
}

#[test]
fn additional_spells_skips_choose_filters_and_daily_wrappers() {
    // Nature domain's bonus cantrip is a `{"choose": ...}` filter (dynamic
    // pick, not a fixed grant); Fathomless's "known" entry is a `{"daily":
    // ...}` wrapper (once-per-day, not an always-active spell). Neither
    // should produce a GrantedSpellRef, but the sibling fixed `prepared`
    // entries in the same block still should.
    let raw = cleric_shaped_file(json!([{
        "known": {"1": {"_": [{"choose": "level=0|class=Druid"}]}, "10": {"daily": {"1": ["evard's black tentacles"]}}},
        "prepared": {"1": ["animal friendship"]}
    }]));

    let bundles = class_bundles_from_parsed(parse_class_file(&raw).unwrap()).unwrap();
    let (_subclass, _features, grants) = &bundles[0].subclasses[0];
    assert_eq!(
        grants,
        &vec![GrantedSpellRef { spell_name_lower: "animal friendship".to_string(), level: 1 }]
    );
}

#[test]
fn additional_spells_with_no_block_produces_no_grants() {
    let raw = cleric_shaped_file(json!([]));
    let bundles = class_bundles_from_parsed(parse_class_file(&raw).unwrap()).unwrap();
    let (_subclass, _features, grants) = &bundles[0].subclasses[0];
    assert!(grants.is_empty());
}

#[test]
fn named_additional_spells_variants_fan_out_into_separate_subclasses() {
    // Mirrors Druid Circle of the Land: every entry is named (a terrain),
    // so this fans out into one Subclass per terrain instead of a single
    // "Circle of the Land" subclass with an unmodeled sub-choice.
    let raw = cleric_shaped_file(json!([
        {"name": "Arctic", "prepared": {"3": ["hold person"]}},
        {"name": "Desert", "prepared": {"3": ["blur"]}}
    ]));

    let bundles = class_bundles_from_parsed(parse_class_file(&raw).unwrap()).unwrap();
    let subclasses = &bundles[0].subclasses;
    assert_eq!(subclasses.len(), 2);

    let mut by_name: std::collections::HashMap<&str, &Vec<GrantedSpellRef>> = std::collections::HashMap::new();
    for (subclass, _features, grants) in subclasses {
        by_name.insert(subclass.name.as_str(), grants);
    }

    let arctic_grants = by_name.get("Life Domain (Arctic)").expect("Arctic variant present");
    assert_eq!(
        arctic_grants.as_slice(),
        &[GrantedSpellRef { spell_name_lower: "hold person".to_string(), level: 3 }]
    );
    let desert_grants = by_name.get("Life Domain (Desert)").expect("Desert variant present");
    assert_eq!(
        desert_grants.as_slice(),
        &[GrantedSpellRef { spell_name_lower: "blur".to_string(), level: 3 }]
    );

    // Each fanned variant gets its own slugified id (distinct short_name).
    let ids: std::collections::HashSet<&str> =
        subclasses.iter().map(|(s, _, _)| s.id.as_str()).collect();
    assert_eq!(ids.len(), 2);
}

#[test]
fn named_variants_inherit_any_ungrouped_common_grants() {
    let raw = cleric_shaped_file(json!([
        {"prepared": {"1": ["bless"]}},
        {"name": "Arctic", "prepared": {"3": ["hold person"]}}
    ]));

    let bundles = class_bundles_from_parsed(parse_class_file(&raw).unwrap()).unwrap();
    let subclasses = &bundles[0].subclasses;
    assert_eq!(subclasses.len(), 1);
    let (subclass, _features, grants) = &subclasses[0];
    assert_eq!(subclass.name, "Life Domain (Arctic)");
    let mut sorted = grants.clone();
    sorted.sort_by_key(|g| g.level);
    assert_eq!(
        sorted,
        vec![
            GrantedSpellRef { spell_name_lower: "bless".to_string(), level: 1 },
            GrantedSpellRef { spell_name_lower: "hold person".to_string(), level: 3 },
        ]
    );
}
