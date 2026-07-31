use super::*;

#[test]
fn no_prerequisites_is_none() {
    assert_eq!(prerequisite_label(&[]), None);
}

#[test]
fn level_and_class_combine_into_one_phrase() {
    let prereqs = vec![PrerequisiteOption {
        level: Some(9),
        class_name: Some("Warlock".to_string()),
        ..Default::default()
    }];
    assert_eq!(prerequisite_label(&prereqs), Some("Warlock level 9".to_string()));
}

#[test]
fn level_class_and_subclass_combine() {
    let prereqs = vec![PrerequisiteOption {
        level: Some(17),
        class_name: Some("Monk".to_string()),
        subclass_name: Some("Four Elements".to_string()),
        ..Default::default()
    }];
    assert_eq!(
        prerequisite_label(&prereqs),
        Some("Monk (Four Elements) level 17".to_string())
    );
}

#[test]
fn pact_and_level_within_one_option_are_comma_joined() {
    let prereqs = vec![PrerequisiteOption {
        level: Some(5),
        class_name: Some("Warlock".to_string()),
        pact: Some("Blade".to_string()),
        ..Default::default()
    }];
    assert_eq!(
        prerequisite_label(&prereqs),
        Some("Warlock level 5, Pact of the Blade".to_string())
    );
}

#[test]
fn required_spells_render_known_suffix() {
    let prereqs = vec![PrerequisiteOption {
        spells: vec!["eldritch blast".to_string()],
        ..Default::default()
    }];
    assert_eq!(prerequisite_label(&prereqs), Some("eldritch blast known".to_string()));
}

#[test]
fn multiple_options_are_or_joined() {
    let prereqs = vec![
        PrerequisiteOption { pact: Some("Chain".to_string()), ..Default::default() },
        PrerequisiteOption {
            level: Some(12),
            class_name: Some("Warlock".to_string()),
            pact: Some("Talisman".to_string()),
            ..Default::default()
        },
    ];
    assert_eq!(
        prerequisite_label(&prereqs),
        Some("Pact of the Chain or Warlock level 12, Pact of the Talisman".to_string())
    );
}

#[test]
fn other_freeform_requirement_renders_verbatim() {
    let prereqs = vec![PrerequisiteOption {
        other: Some("A suit of armor (requires attunement)".to_string()),
        ..Default::default()
    }];
    assert_eq!(
        prerequisite_label(&prereqs),
        Some("A suit of armor (requires attunement)".to_string())
    );
}

#[test]
fn empty_option_contributes_nothing() {
    let prereqs = vec![PrerequisiteOption::default()];
    assert_eq!(prerequisite_label(&prereqs), None);
}

#[test]
fn class_requirement_label_combines_class_and_subclass() {
    let bare_class = PrerequisiteOption { class_name: Some("Warlock".to_string()), ..Default::default() };
    assert_eq!(bare_class.class_requirement_label(), Some("Warlock".to_string()));

    let with_subclass = PrerequisiteOption {
        class_name: Some("Monk".to_string()),
        subclass_name: Some("Four Elements".to_string()),
        ..Default::default()
    };
    assert_eq!(with_subclass.class_requirement_label(), Some("Monk (Four Elements)".to_string()));

    let no_class = PrerequisiteOption::default();
    assert_eq!(no_class.class_requirement_label(), None);
}

#[test]
fn class_requirement_labels_dedupes_and_sorts_across_options() {
    let prereqs = vec![
        PrerequisiteOption { class_name: Some("Warlock".to_string()), ..Default::default() },
        PrerequisiteOption { pact: Some("Chain".to_string()), ..Default::default() }, // no class, contributes nothing
        PrerequisiteOption { class_name: Some("Warlock".to_string()), ..Default::default() }, // duplicate
        PrerequisiteOption {
            class_name: Some("Monk".to_string()),
            subclass_name: Some("Four Elements".to_string()),
            ..Default::default()
        },
    ];
    assert_eq!(
        class_requirement_labels(&prereqs),
        vec!["Monk (Four Elements)".to_string(), "Warlock".to_string()]
    );
}

#[test]
fn required_pacts_dedupes_and_sorts_across_options() {
    let prereqs = vec![
        PrerequisiteOption { pact: Some("Chain".to_string()), ..Default::default() },
        PrerequisiteOption { class_name: Some("Warlock".to_string()), ..Default::default() }, // no pact
        PrerequisiteOption { pact: Some("Chain".to_string()), ..Default::default() }, // duplicate
        PrerequisiteOption { pact: Some("Blade".to_string()), ..Default::default() },
    ];
    assert_eq!(required_pacts(&prereqs), vec!["Blade".to_string(), "Chain".to_string()]);
}

#[test]
fn feature_type_from_code_round_trips_and_tolerates_unknown_codes() {
    assert_eq!(FeatureType::from_code("EI"), Some(FeatureType::EldritchInvocation));
    assert_eq!(FeatureType::from_code("MV:B"), Some(FeatureType::Maneuver));
    assert_eq!(FeatureType::from_code("FS:B"), Some(FeatureType::FightingStyleBard));
    assert_eq!(FeatureType::from_code("NOPE"), None);
}

#[test]
fn feature_type_as_str_and_from_str_key_round_trip_for_every_variant() {
    for feature_type in FeatureType::ALL {
        assert_eq!(FeatureType::from_str_key(feature_type.as_str()), Some(feature_type));
    }
    assert_eq!(FeatureType::from_str_key("not-a-real-key"), None);
}

#[test]
fn group_label_collapses_the_four_fighting_styles_and_otherwise_matches_label() {
    for feature_type in [
        FeatureType::FightingStyleFighter,
        FeatureType::FightingStyleRanger,
        FeatureType::FightingStylePaladin,
        FeatureType::FightingStyleBard,
    ] {
        assert_eq!(feature_type.group_label(), "Fighting Style");
    }
    // Every other variant just drops the label's "(Class)" suffix.
    assert_eq!(FeatureType::EldritchInvocation.group_label(), "Eldritch Invocation");
    assert_eq!(FeatureType::PactBoon.group_label(), "Pact Boon");
    assert_eq!(FeatureType::Maneuver.group_label(), "Maneuver");
}

fn ctx<'a>(
    level: u8,
    class_name: &'a str,
    subclass_name: Option<&'a str>,
    known_spell_names: &'a HashSet<String>,
    chosen_feature_names: &'a HashSet<String>,
) -> EligibilityContext<'a> {
    EligibilityContext {
        level,
        class_name,
        class_source: "PHB",
        subclass_name,
        subclass_source: subclass_name.map(|_| "PHB"),
        known_spell_names,
        chosen_feature_names,
    }
}

#[test]
fn is_eligible_with_no_prerequisites_is_always_true() {
    let empty = HashSet::new();
    assert!(is_eligible(&[], &ctx(1, "Warlock", None, &empty, &empty)));
}

#[test]
fn is_eligible_checks_level_only_when_no_class_named() {
    let prereqs = vec![PrerequisiteOption { level: Some(9), ..Default::default() }];
    let empty = HashSet::new();
    assert!(!is_eligible(&prereqs, &ctx(8, "Warlock", None, &empty, &empty)));
    assert!(is_eligible(&prereqs, &ctx(9, "Warlock", None, &empty, &empty)));
}

#[test]
fn is_eligible_requires_matching_class_name_case_insensitively() {
    let prereqs = vec![PrerequisiteOption {
        level: Some(9),
        class_name: Some("Warlock".to_string()),
        ..Default::default()
    }];
    let empty = HashSet::new();
    assert!(!is_eligible(&prereqs, &ctx(9, "Fighter", None, &empty, &empty)));
    assert!(is_eligible(&prereqs, &ctx(9, "warlock", None, &empty, &empty)));
}

#[test]
fn is_eligible_requires_matching_subclass_when_named() {
    let prereqs = vec![PrerequisiteOption {
        level: Some(17),
        class_name: Some("Monk".to_string()),
        subclass_name: Some("Four Elements".to_string()),
        ..Default::default()
    }];
    let empty = HashSet::new();
    assert!(!is_eligible(&prereqs, &ctx(17, "Monk", None, &empty, &empty)));
    assert!(!is_eligible(&prereqs, &ctx(17, "Monk", Some("Shadow"), &empty, &empty)));
    assert!(is_eligible(&prereqs, &ctx(17, "Monk", Some("Four Elements"), &empty, &empty)));
}

#[test]
fn is_eligible_checks_pact_against_chosen_feature_names() {
    let prereqs = vec![PrerequisiteOption {
        level: Some(5),
        class_name: Some("Warlock".to_string()),
        pact: Some("Blade".to_string()),
        ..Default::default()
    }];
    let empty = HashSet::new();
    let mut has_blade = HashSet::new();
    has_blade.insert("pact of the blade".to_string());

    assert!(!is_eligible(&prereqs, &ctx(5, "Warlock", None, &empty, &empty)));
    assert!(is_eligible(&prereqs, &ctx(5, "Warlock", None, &empty, &has_blade)));
}

#[test]
fn is_eligible_checks_any_required_spell_is_known() {
    let prereqs = vec![PrerequisiteOption { spells: vec!["eldritch blast".to_string()], ..Default::default() }];
    let empty = HashSet::new();
    let mut knows_blast = HashSet::new();
    knows_blast.insert("eldritch blast".to_string());

    assert!(!is_eligible(&prereqs, &ctx(1, "Warlock", None, &empty, &empty)));
    assert!(is_eligible(&prereqs, &ctx(1, "Warlock", None, &knows_blast, &empty)));
}

#[test]
fn is_eligible_treats_freeform_other_as_unverifiable_but_satisfied() {
    let prereqs = vec![PrerequisiteOption {
        other: Some("A suit of armor (requires attunement)".to_string()),
        ..Default::default()
    }];
    let empty = HashSet::new();
    assert!(is_eligible(&prereqs, &ctx(1, "Artificer", None, &empty, &empty)));
}

#[test]
fn is_eligible_is_true_if_any_or_alternative_is_satisfied() {
    let prereqs = vec![
        PrerequisiteOption { pact: Some("Chain".to_string()), ..Default::default() },
        PrerequisiteOption {
            level: Some(12),
            class_name: Some("Warlock".to_string()),
            pact: Some("Talisman".to_string()),
            ..Default::default()
        },
    ];
    let empty = HashSet::new();
    let mut has_talisman = HashSet::new();
    has_talisman.insert("pact of the talisman".to_string());

    // Neither alternative satisfied.
    assert!(!is_eligible(&prereqs, &ctx(12, "Warlock", None, &empty, &empty)));
    // Second alternative satisfied is enough.
    assert!(is_eligible(&prereqs, &ctx(12, "Warlock", None, &empty, &has_talisman)));
}
