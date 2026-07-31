use super::*;
use crate::models::skill::SkillGrant;
use serde_json::json;

#[test]
fn fixed_skill_grants_are_captured_verbatim() {
    assert_eq!(
        skill_grants_from_background(&[json!({ "insight": true, "sleight of hand": true })]),
        vec![SkillGrant::Fixed {
            skills: vec!["insight".to_string(), "sleight of hand".to_string()]
        }]
    );
}

#[test]
fn any_standard_skill_grant_becomes_an_any_grant() {
    assert_eq!(
        skill_grants_from_background(&[json!({ "anyStandard": 2 })]),
        vec![SkillGrant::Any { count: 2 }]
    );
}

#[test]
fn choose_skill_grants_keep_their_options_and_count() {
    assert_eq!(
        skill_grants_from_background(&[json!({
            "choose": { "from": ["arcana", "nature"], "count": 2 }
        })]),
        vec![SkillGrant::Choose { count: 2, from: vec!["arcana".to_string(), "nature".to_string()] }]
    );
    assert_eq!(
        skill_grants_from_background(&[json!({ "choose": { "from": ["arcana"] } })]),
        vec![SkillGrant::Choose { count: 1, from: vec!["arcana".to_string()] }]
    );
}

#[test]
fn wildcard_tool_grants_show_counts() {
    assert_eq!(
        proficiency_labels(&[json!({ "anyArtisansTool": 1, "anyGamingSet": 2 })]),
        vec!["Any artisan's tools", "Any 2 gaming set"]
    );
}

#[test]
fn title_case_keeps_connecting_words_lowercase() {
    assert_eq!(title_case("sleight of hand"), "Sleight of Hand");
    assert_eq!(title_case("vehicles (land)"), "Vehicles (land)");
    assert_eq!(title_case("of note"), "Of Note");
}
