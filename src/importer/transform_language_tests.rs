use super::*;

fn raw(name: &str, source: &str, language_type: Option<&str>, script: Option<&str>) -> RawLanguage {
    RawLanguage {
        name: name.to_string(),
        source: source.to_string(),
        language_type: language_type.map(str::to_string),
        script: script.map(str::to_string),
    }
}

#[test]
fn standard_language_carries_type_and_script() {
    let language = language_from_raw(raw("Common", "PHB", Some("standard"), Some("Common")));
    assert_eq!(language.name, "Common");
    assert_eq!(language.id, "common-phb");
    assert_eq!(language.canonical_id, "Common|PHB");
    assert_eq!(language.language_type, Some(LanguageType::Standard));
    assert_eq!(language.script, Some("Common".to_string()));
}

#[test]
fn exotic_and_rare_and_secret_types_parse() {
    assert_eq!(
        language_from_raw(raw("Abyssal", "GGR", Some("exotic"), Some("Infernal"))).language_type,
        Some(LanguageType::Exotic)
    );
    assert_eq!(
        language_from_raw(raw("Loross", "GGR", Some("rare"), None)).language_type,
        Some(LanguageType::Rare)
    );
    assert_eq!(
        language_from_raw(raw("Thieves' Cant", "PHB", Some("secret"), None)).language_type,
        Some(LanguageType::Secret)
    );
}

#[test]
fn same_name_different_source_gets_distinct_ids() {
    // Common/Draconic/etc. are reprinted across a dozen+ source books —
    // the id must include the source or reseeding real fixtures collides
    // on a UNIQUE constraint.
    let phb = language_from_raw(raw("Abyssal", "GGR", Some("exotic"), None));
    let mtf = language_from_raw(raw("Abyssal", "MTF", Some("exotic"), None));
    assert_ne!(phb.id, mtf.id);
}

#[test]
fn monster_only_language_has_no_type() {
    let language = language_from_raw(raw("Aarakocra", "MM", None, None));
    assert_eq!(language.language_type, None);
    assert_eq!(language.script, None);
}
