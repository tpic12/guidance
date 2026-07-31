use super::*;
use crate::importer::parse::{RawMeta, RawTime};

fn raw_spell(name: &str) -> RawSpell {
    RawSpell {
        name: name.to_string(),
        source: "PHB".to_string(),
        level: 1,
        school: "A".to_string(),
        time: vec![],
        range: None,
        components: RawComponents::default(),
        duration: vec![],
        meta: RawMeta::default(),
        entries: vec![],
        entries_higher_level: vec![],
    }
}

#[test]
fn slugify_normalizes_names() {
    assert_eq!(slugify("Fireball"), "fireball");
    assert_eq!(slugify("Melf's Acid Arrow"), "melf-s-acid-arrow");
    assert_eq!(slugify("  Antipathy/Sympathy  "), "antipathy-sympathy");
}

#[test]
fn unknown_school_code_is_an_error() {
    let mut raw = raw_spell("Mystery");
    raw.school = "Z".to_string();
    let err = spells_from_parsed(vec![raw]).unwrap_err();
    assert!(err.to_string().contains("Mystery"), "error should name the spell: {err}");
}

#[test]
fn missing_time_range_and_duration_get_sensible_defaults() {
    let spell = spells_from_parsed(vec![raw_spell("Shield")]).unwrap().remove(0);
    assert_eq!(spell.casting_time, 1);
    assert_eq!(spell.casting_type, CastingType::Action);
    assert_eq!(spell.range.kind, "self");
    assert_eq!(spell.duration.kind, "instant");
    assert_eq!(spell.canonical_id, "Shield|PHB");
}

#[test]
fn reaction_spells_keep_their_trigger_condition() {
    let mut raw = raw_spell("Counterspell");
    raw.time = vec![RawTime {
        number: 1,
        unit: "reaction".to_string(),
        condition: Some("which you take when you see a creature casting".to_string()),
    }];
    let spell = spells_from_parsed(vec![raw]).unwrap().remove(0);
    assert_eq!(spell.casting_type, CastingType::Reaction);
    assert_eq!(
        spell.casting_condition.as_deref(),
        Some("which you take when you see a creature casting")
    );
}
