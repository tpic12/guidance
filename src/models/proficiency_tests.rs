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
        "Choose 2 from Musical Instrument, Gaming Set"
    );
}

#[test]
fn contains_ignore_case_matches_regardless_of_casing() {
    let fixed = vec!["common".to_string(), "alchemist's supplies".to_string()];
    assert!(contains_ignore_case(&fixed, "Common"));
    assert!(contains_ignore_case(&fixed, "Alchemist's Supplies"));
    assert!(contains_ignore_case(&fixed, "alchemist's supplies"));
    assert!(!contains_ignore_case(&fixed, "Draconic"));
}

#[test]
fn merge_resolved_names_dedupes_a_fixed_grant_against_a_differently_cased_choice() {
    // Real shape: a species fixed-grants "common" (raw import key) and the
    // player separately picks "Common" (DB-sourced) via an Any/Choose slot
    // — these must collapse into one entry, not render as a visible duplicate.
    let fixed = vec!["common".to_string(), "auran".to_string()];
    let choices = vec!["Common".to_string(), "Draconic".to_string()];
    assert_eq!(
        merge_resolved_names(&fixed, &choices),
        vec!["Auran".to_string(), "Common".to_string(), "Draconic".to_string()]
    );
}

#[test]
fn merge_resolved_names_title_cases_a_fixed_only_entry() {
    assert_eq!(merge_resolved_names(&["thieves' tools".to_string()], &[]), vec!["Thieves' Tools".to_string()]);
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
