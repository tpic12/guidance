use super::*;
use crate::models::language::LanguageGrant;

#[test]
fn tool_fixed_grant_lists_title_cased_names() {
    assert_eq!(
        describe_tool_grants(&[ToolGrant::Fixed {
            tools: vec!["thieves' tools".to_string(), "tinker's tools".to_string()]
        }]),
        "Thieves' Tools, Tinker's Tools"
    );
}

#[test]
fn tool_any_grant_renders_count() {
    assert_eq!(describe_tool_grants(&[ToolGrant::Any { count: 3 }]), "Any 3");
}

#[test]
fn tool_any_category_grant_renders_label() {
    assert_eq!(
        describe_tool_grants(&[ToolGrant::AnyCategory { count: 1, category: ToolCategory::ArtisansTool }]),
        "Any artisan's tools"
    );
    assert_eq!(
        describe_tool_grants(&[ToolGrant::AnyCategory { count: 2, category: ToolCategory::MusicalInstrument }]),
        "Any 2 musical instrument"
    );
}

#[test]
fn tool_choose_grant_mixes_named_and_category_options() {
    assert_eq!(
        describe_tool_grants(&[ToolGrant::Choose {
            count: 2,
            from: vec![
                ToolOption::Category(ToolCategory::MusicalInstrument),
                ToolOption::Named("gaming set".to_string()),
            ]
        }]),
        "Choose 2 from musical instrument, Gaming Set"
    );
}

#[test]
fn tool_category_item_type_codes() {
    assert_eq!(ToolCategory::ArtisansTool.item_type_code(), "AT");
    assert_eq!(ToolCategory::GamingSet.item_type_code(), "GS");
    assert_eq!(ToolCategory::MusicalInstrument.item_type_code(), "INS");
}

#[test]
fn language_fixed_and_any_grants_render() {
    assert_eq!(
        describe_language_grants(&[LanguageGrant::Fixed { languages: vec!["common".to_string()] }]),
        "Common"
    );
    assert_eq!(describe_language_grants(&[LanguageGrant::Any { count: 2 }]), "Any 2");
}

#[test]
fn language_choose_grant_renders() {
    assert_eq!(
        describe_language_grants(&[LanguageGrant::Choose {
            count: 1,
            from: vec!["auran".to_string(), "aquan".to_string()]
        }]),
        "Choose 1 from Auran, Aquan"
    );
}
