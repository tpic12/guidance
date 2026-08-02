use super::{group_species_by_name, optional_feature_cost_label, species_subtitle};
use crate::models::optional_feature::ResourceCost;
use crate::models::species::Species;

fn species(id: &str, name: &str, source: &str) -> Species {
    Species {
        id: id.to_string(),
        canonical_id: format!("{name}|{source}"),
        name: name.to_string(),
        source: source.to_string(),
        ability: None,
        ability_bonuses: vec![],
        size: None,
        speed: None,
        darkvision: None,
        entries: vec![],
    }
}

#[test]
fn group_species_by_name_keeps_unique_species_as_singleton_groups() {
    let rows = vec![species("elf", "Elf", "PHB"), species("dwarf", "Dwarf", "PHB")];
    let groups = group_species_by_name(rows);
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].0, "Elf");
    assert_eq!(groups[0].1.len(), 1);
    assert_eq!(groups[1].0, "Dwarf");
    assert_eq!(groups[1].1.len(), 1);
}

#[test]
fn group_species_by_name_collapses_reprints_preserving_first_occurrence_order() {
    let rows = vec![
        species("elf-phb", "Elf", "PHB"),
        species("dwarf-phb", "Dwarf", "PHB"),
        species("elf-tbk", "Elf", "TBK"),
    ];
    let groups = group_species_by_name(rows);
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].0, "Elf");
    assert_eq!(groups[0].1.iter().map(|s| s.source.as_str()).collect::<Vec<_>>(), vec!["PHB", "TBK"]);
    assert_eq!(groups[1].0, "Dwarf");
    assert_eq!(groups[1].1.len(), 1);
}

#[test]
fn species_subtitle_joins_only_present_fields() {
    let mut s = species("elf", "Elf", "PHB");
    s.ability = Some("Dexterity +2".to_string());
    s.size = Some("Medium".to_string());
    s.speed = None;
    assert_eq!(species_subtitle(&s), "Dexterity +2 · Medium");
}

#[test]
fn species_subtitle_is_empty_when_no_fields_are_present() {
    let s = species("elf", "Elf", "PHB");
    assert_eq!(species_subtitle(&s), "");
}

fn cost(name: &str, amount: Option<u32>, min: Option<u32>, max: Option<u32>) -> ResourceCost {
    ResourceCost { name: name.to_string(), amount, amount_min: min, amount_max: max }
}

#[test]
fn optional_feature_cost_label_uses_exact_amount_when_present() {
    assert_eq!(optional_feature_cost_label(&cost("Ki Points", Some(3), Some(1), Some(5))), "3 Ki Points");
}

#[test]
fn optional_feature_cost_label_uses_range_when_no_exact_amount() {
    assert_eq!(optional_feature_cost_label(&cost("Sorcery Points", None, Some(2), Some(4))), "2-4 Sorcery Points");
}

#[test]
fn optional_feature_cost_label_uses_open_ended_minimum() {
    assert_eq!(optional_feature_cost_label(&cost("Charges", None, Some(1), None)), "1+ Charges");
}

#[test]
fn optional_feature_cost_label_omits_amount_when_none_given() {
    assert_eq!(optional_feature_cost_label(&cost("Rage", None, None, None)), "Rage");
}
