use super::*;

#[test]
fn skill_ability_maps_known_skills() {
    assert_eq!(skill_ability("athletics"), Some("str"));
    assert_eq!(skill_ability("sleight of hand"), Some("dex"));
    assert_eq!(skill_ability("not-a-skill"), None);
}

#[test]
fn is_valid_skill_rejects_unknown_names() {
    assert!(is_valid_skill("stealth"));
    assert!(!is_valid_skill("Stealth"));
    assert!(!is_valid_skill("lockpicking"));
}

#[test]
fn skill_label_capitalizes_and_keeps_connectors_lowercase() {
    assert_eq!(skill_label("athletics"), "Athletics");
    assert_eq!(skill_label("animal handling"), "Animal Handling");
    assert_eq!(skill_label("sleight of hand"), "Sleight of Hand");
}

#[test]
fn describe_skill_grants_renders_each_variant() {
    let grants = vec![
        SkillGrant::Fixed { skills: vec!["insight".to_string(), "history".to_string()] },
        SkillGrant::Choose {
            count: 2,
            from: vec!["athletics".to_string(), "survival".to_string()],
        },
        SkillGrant::Any { count: 1 },
    ];
    assert_eq!(
        describe_skill_grants(&grants),
        "Insight, History; Choose 2 from Athletics, Survival; Any 1"
    );
}

#[test]
fn describe_skill_grants_empty_is_empty_string() {
    assert_eq!(describe_skill_grants(&[]), "");
}
