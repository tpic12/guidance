use super::*;
use serde_json::json;

#[test]
fn artificer_mixes_fixed_and_any_category() {
    let grants = tool_grants_from_raw(&[json!({
        "thieves' tools": true,
        "tinker's tools": true,
        "anyArtisansTool": 1
    })]);
    assert_eq!(
        grants,
        vec![
            ToolGrant::Fixed { tools: vec!["thieves' tools".to_string(), "tinker's tools".to_string()] },
            ToolGrant::AnyCategory { count: 1, category: ToolCategory::ArtisansTool },
        ]
    );
}

#[test]
fn bard_any_musical_instrument() {
    let grants = tool_grants_from_raw(&[json!({ "anyMusicalInstrument": 3 })]);
    assert_eq!(grants, vec![ToolGrant::AnyCategory { count: 3, category: ToolCategory::MusicalInstrument }]);
}

#[test]
fn far_traveler_choose_mixes_named_and_category_tokens() {
    let grants = tool_grants_from_raw(&[json!({
        "choose": { "from": ["musical instrument", "gaming set"] }
    })]);
    assert_eq!(
        grants,
        vec![ToolGrant::Choose {
            count: 1,
            from: vec![
                ToolOption::Category(ToolCategory::MusicalInstrument),
                ToolOption::Category(ToolCategory::GamingSet),
            ]
        }]
    );
}

#[test]
fn urban_bounty_hunter_choose_two_from_mixed_list() {
    let grants = tool_grants_from_raw(&[json!({
        "choose": { "from": ["gaming set", "musical instrument", "thieves' tools"], "count": 2 }
    })]);
    assert_eq!(
        grants,
        vec![ToolGrant::Choose {
            count: 2,
            from: vec![
                ToolOption::Category(ToolCategory::GamingSet),
                ToolOption::Category(ToolCategory::MusicalInstrument),
                ToolOption::Named("thieves' tools".to_string()),
            ]
        }]
    );
}

#[test]
fn acolyte_any_standard_language() {
    let grants = language_grants_from_raw(&[json!({ "anyStandard": 2 })]);
    assert_eq!(grants, vec![LanguageGrant::Any { count: 2 }]);
}

#[test]
fn aarakocra_fixed_language_only() {
    let grants = language_grants_from_raw(&[json!({ "auran": true })]);
    assert_eq!(grants, vec![LanguageGrant::Fixed { languages: vec!["auran".to_string()] }]);
}

#[test]
fn merfolk_mixes_fixed_other_and_any_standard() {
    let grants = language_grants_from_raw(&[json!({
        "common": true,
        "other": true,
        "anyStandard": 1
    })]);
    assert_eq!(
        grants,
        vec![
            LanguageGrant::Fixed { languages: vec!["common".to_string()] },
            LanguageGrant::Any { count: 1 },
            LanguageGrant::Any { count: 1 },
        ]
    );
}
