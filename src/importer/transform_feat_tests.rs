use super::*;
use serde_json::json;

#[test]
fn no_prerequisites_is_none() {
    assert_eq!(prerequisite_label(&[]), None);
}

#[test]
fn requirements_within_one_option_are_comma_joined() {
    assert_eq!(
        prerequisite_label(&[json!({ "level": 4, "race": [{ "name": "dwarf" }] })]),
        Some("4th level, Dwarf".to_string())
    );
}

#[test]
fn alternative_options_are_or_joined() {
    assert_eq!(
        prerequisite_label(&[
            json!({ "ability": [{ "str": 13 }] }),
            json!({ "ability": [{ "dex": 13 }] })
        ]),
        Some("Strength 13 or higher or Dexterity 13 or higher".to_string())
    );
}

#[test]
fn shared_score_groups_ability_names() {
    assert_eq!(
        prerequisite_label(&[json!({ "ability": [{ "str": 13 }, { "dex": 13 }] })]),
        Some("Strength or Dexterity 13 or higher".to_string())
    );
}

#[test]
fn proficiency_spellcasting_and_other_render() {
    assert_eq!(
        prerequisite_label(&[json!({ "proficiency": [{ "armor": "medium" }] })]),
        Some("Proficiency with medium armor".to_string())
    );
    assert_eq!(
        prerequisite_label(&[json!({ "spellcasting": true })]),
        Some("The ability to cast at least one spell".to_string())
    );
    assert_eq!(
        prerequisite_label(&[json!({ "other": "No other dragonmark" })]),
        Some("No other dragonmark".to_string())
    );
}

#[test]
fn feat_prerequisites_prefer_the_display_name() {
    assert_eq!(
        prerequisite_label(&[json!({
            "feat": ["initiate of high sorcery|dsotdq|initiate of high sorcery (nuitari)"]
        })]),
        Some("Initiate of High Sorcery (nuitari) feat".to_string())
    );
    assert_eq!(
        prerequisite_label(&[json!({ "feat": ["grappler|phb"] })]),
        Some("Grappler feat".to_string())
    );
}

#[test]
fn fixed_ability_increase_renders() {
    assert_eq!(
        ability_increase_label(&[json!({ "con": 1 })]),
        Some("Increase your Constitution score by 1, to a maximum of 20.".to_string())
    );
}

#[test]
fn choose_across_all_six_abilities_is_a_free_choice() {
    assert_eq!(
        ability_increase_label(&[json!({
            "choose": { "from": ["str", "dex", "con", "int", "wis", "cha"], "amount": 1 }
        })]),
        Some("Increase one ability score of your choice by 1, to a maximum of 20.".to_string())
    );
}

#[test]
fn choose_from_a_subset_names_the_options() {
    assert_eq!(
        ability_increase_label(&[json!({ "choose": { "from": ["str", "dex"], "amount": 1 } })]),
        Some("Increase your Strength or Dexterity score by 1, to a maximum of 20.".to_string())
    );
}
