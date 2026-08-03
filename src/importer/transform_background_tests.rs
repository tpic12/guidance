use super::*;
use crate::models::language::LanguageGrant;
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
fn background_from_raw_wires_structured_language_and_tool_grants() {
    // Mirrors test-fixtures/backgrounds.json's Fake Scholar/Fake Wanderer shapes.
    let raw = RawBackground {
        name: "Fake Scholar".to_string(),
        source: "TBK".to_string(),
        skill_proficiencies: vec![json!({ "history": true, "insight": true })],
        language_proficiencies: vec![json!({ "anyStandard": 2 })],
        tool_proficiencies: vec![],
        entries: vec![],
    };
    let background = background_from_raw(raw);
    assert_eq!(background.languages, vec![LanguageGrant::Any { count: 2 }]);
    assert_eq!(background.tools, vec![]);
}

#[test]
fn background_from_raw_wires_mixed_category_tool_grants() {
    // Mirrors Far Traveler's real toolProficiencies shape.
    let raw = RawBackground {
        name: "Fake Wanderer".to_string(),
        source: "ZBK".to_string(),
        skill_proficiencies: vec![],
        language_proficiencies: vec![],
        tool_proficiencies: vec![json!({ "choose": { "from": ["musical instrument", "gaming set"] } })],
        entries: vec![],
    };
    let background = background_from_raw(raw);
    assert_eq!(
        background.tools,
        vec![crate::models::proficiency::ToolGrant::Choose {
            count: 1,
            from: vec![
                crate::models::proficiency::ToolOption::Category(
                    crate::models::proficiency::ToolCategory::MusicalInstrument
                ),
                crate::models::proficiency::ToolOption::Category(
                    crate::models::proficiency::ToolCategory::GamingSet
                ),
            ]
        }]
    );
}

#[test]
fn title_case_keeps_connecting_words_lowercase() {
    assert_eq!(title_case("sleight of hand"), "Sleight of Hand");
    assert_eq!(title_case("vehicles (land)"), "Vehicles (land)");
    assert_eq!(title_case("of note"), "Of Note");
}
