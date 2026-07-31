use super::*;
use crate::models::species::AbilityBonusGrant;
use serde_json::json;

#[test]
fn no_ability_increase_is_none() {
    assert_eq!(ability_score_label(&[]), None);
}

#[test]
fn no_ability_grants_is_empty() {
    assert_eq!(ability_bonus_grants(&[]), Vec::new());
}

#[test]
fn fixed_ability_grants_are_structured() {
    assert_eq!(
        ability_bonus_grants(&[json!({ "dex": 2, "wis": 1 })]),
        vec![
            AbilityBonusGrant::Fixed { code: "dex".to_string(), amount: 2 },
            AbilityBonusGrant::Fixed { code: "wis".to_string(), amount: 1 },
        ]
    );
}

#[test]
fn negative_fixed_ability_grants_are_preserved() {
    assert_eq!(
        ability_bonus_grants(&[json!({ "int": -2 })]),
        vec![AbilityBonusGrant::Fixed { code: "int".to_string(), amount: -2 }]
    );
}

#[test]
fn choose_grant_combines_with_a_fixed_grant() {
    assert_eq!(
        ability_bonus_grants(&[json!({
            "cha": 2,
            "choose": { "from": ["str", "dex"], "count": 2 }
        })]),
        vec![
            AbilityBonusGrant::Fixed { code: "cha".to_string(), amount: 2 },
            AbilityBonusGrant::Choose {
                count: 2,
                amount: 1,
                from: vec!["str".to_string(), "dex".to_string()],
            },
        ]
    );
}

#[test]
fn choose_grant_defaults_count_and_amount_to_one() {
    assert_eq!(
        ability_bonus_grants(&[json!({ "choose": { "from": ["str", "dex", "con"] } })]),
        vec![AbilityBonusGrant::Choose {
            count: 1,
            amount: 1,
            from: vec!["str".to_string(), "dex".to_string(), "con".to_string()],
        }]
    );
}

#[test]
fn choose_grant_with_no_from_list_is_a_free_choice() {
    assert_eq!(
        ability_bonus_grants(&[json!({ "choose": { "count": 2 } })]),
        vec![AbilityBonusGrant::Choose { count: 2, amount: 1, from: Vec::new() }]
    );
}

#[test]
fn fixed_ability_scores_are_comma_joined() {
    assert_eq!(
        ability_score_label(&[json!({ "dex": 2, "wis": 1 })]),
        Some("Dexterity +2, Wisdom +1".to_string())
    );
}

#[test]
fn choose_across_all_six_abilities_is_a_free_choice() {
    assert_eq!(
        ability_score_label(&[json!({
            "con": 2,
            "choose": { "from": ["str", "dex", "con", "int", "wis", "cha"] }
        })]),
        Some("Constitution +2, Choose any +1".to_string())
    );
}

#[test]
fn choose_from_a_subset_names_the_options() {
    assert_eq!(
        ability_score_label(&[json!({
            "cha": 2,
            "choose": { "from": ["str", "dex"], "count": 1 }
        })]),
        Some("Charisma +2, Choose Strength or Dexterity +1".to_string())
    );
}

#[test]
fn choose_multiple_lists_the_count() {
    assert_eq!(
        ability_score_label(&[json!({
            "choose": { "from": ["str", "dex", "con"], "count": 2 }
        })]),
        Some("Choose 2 from Strength, Dexterity, Constitution +1".to_string())
    );
}

#[test]
fn bare_speed_is_a_walking_speed() {
    assert_eq!(speed_label(&json!(25)), Some("25 ft.".to_string()));
}

#[test]
fn speed_map_renders_modes_with_true_matching_walk() {
    assert_eq!(
        speed_label(&json!({ "walk": 30, "fly": true, "swim": 20 })),
        Some("30 ft., fly 30 ft., swim 20 ft.".to_string())
    );
}

#[test]
fn missing_speed_is_none() {
    assert_eq!(speed_label(&serde_json::Value::Null), None);
}

#[test]
fn size_codes_render_as_names() {
    assert_eq!(size_label(&[]), None);
    assert_eq!(size_label(&["M".to_string()]), Some("Medium".to_string()));
    assert_eq!(
        size_label(&["S".to_string(), "M".to_string()]),
        Some("Small or Medium".to_string())
    );
    assert_eq!(size_label(&["V".to_string()]), Some("Varies".to_string()));
}

#[test]
fn copy_stubs_are_skipped() {
    let raws = crate::importer::parse_species::parse_species_file(
        r#"{"race": [
            {"name": "Original", "source": "TBK", "entries": []},
            {"name": "Reprint", "source": "ZBK", "_copy": {"name": "Original", "source": "TBK"}}
        ]}"#,
    )
    .unwrap();
    let species = species_from_parsed(raws).unwrap();
    assert_eq!(species.len(), 1);
    assert_eq!(species[0].name, "Original");
    assert_eq!(species[0].canonical_id, "Original|TBK");
}
