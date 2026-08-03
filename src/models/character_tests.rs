use super::*;
use crate::models::class::{Class, OptionalFeatureProgression, Proficiencies, Subclass, SubclassDetail};
use crate::models::language::LanguageGrant;
use crate::models::optional_feature::FeatureType;
use crate::models::proficiency::{ToolCategory, ToolGrant, ToolOption};
use crate::models::skill::SkillGrant;
use crate::models::species::AbilityBonusGrant;

fn character() -> Character {
    Character {
        id: "test".to_string(),
        user_id: "owner-1".to_string(),
        name: "Test Hero".to_string(),
        species_id: "fake-skyfolk-tbk".to_string(),
        background_id: "fake-acolyte".to_string(),
        classes: vec![ClassLevel { class_id: "fake-warrior".to_string(), subclass_id: None, level: 5 }],
        ability_method: AbilityMethod::Manual,
        abilities: AbilityScores {
            strength: 15,
            dexterity: 14,
            constitution: 13,
            intelligence: 12,
            wisdom: 10,
            charisma: 8,
        },
        asi_choices: vec![],
        skill_choices: vec![],
        expertise_choices: vec![],
        language_choices: vec![],
        tool_choices: vec![],
        custom_skill_proficiencies: vec![],
        custom_language_proficiencies: vec![],
        custom_tool_proficiencies: vec![],
        cantrip_choices: vec![],
        spell_choices: vec![],
        spell_grant_choices: vec![],
        optional_feature_choices: vec![],
        species_ability_choices: vec![],
        ability_bonus_source: AbilityBonusSource::default(),
        custom_ability_bonus_choices: vec![],
        hp_method: HpMethod::Average,
        hp_rolls: vec![],
        hp_manual: None,
        enforce_multiclass_prereqs: true,
    }
}

fn species_with_bonuses(ability_bonuses: Vec<AbilityBonusGrant>) -> Species {
    Species {
        id: "fake-skyfolk-tbk".to_string(),
        canonical_id: "Fake Skyfolk|TBK".to_string(),
        name: "Fake Skyfolk".to_string(),
        source: "TBK".to_string(),
        ability: None,
        ability_bonuses,
        size: None,
        speed: None,
        darkvision: None,
        languages: vec![],
        entries: vec![],
    }
}

fn feature(name: &str, level: u8) -> ClassFeature {
    ClassFeature {
        name: name.to_string(),
        source: "TBK".to_string(),
        level,
        entries: vec![],
    }
}

#[test]
fn ability_modifiers_floor_correctly() {
    assert_eq!(ability_modifier(8), -1);
    assert_eq!(ability_modifier(9), -1);
    assert_eq!(ability_modifier(10), 0);
    assert_eq!(ability_modifier(11), 0);
    assert_eq!(ability_modifier(15), 2);
    assert_eq!(ability_modifier(20), 5);
    assert_eq!(ability_modifier(3), -4);
}

#[test]
fn proficiency_bonus_breakpoints() {
    assert_eq!(proficiency_bonus(1), 2);
    assert_eq!(proficiency_bonus(4), 2);
    assert_eq!(proficiency_bonus(5), 3);
    assert_eq!(proficiency_bonus(8), 3);
    assert_eq!(proficiency_bonus(9), 4);
    assert_eq!(proficiency_bonus(13), 5);
    assert_eq!(proficiency_bonus(17), 6);
    assert_eq!(proficiency_bonus(20), 6);
}

#[test]
fn point_buy_costs_cover_the_buyable_range() {
    assert_eq!(point_buy_cost(8), Some(0));
    assert_eq!(point_buy_cost(13), Some(5));
    assert_eq!(point_buy_cost(15), Some(9));
    assert_eq!(point_buy_cost(7), None);
    assert_eq!(point_buy_cost(16), None);
}

#[test]
fn total_level_sums_every_class() {
    assert_eq!(total_level(&[]), 0);
    let classes = vec![
        ClassLevel { class_id: "fake-warrior".to_string(), subclass_id: None, level: 5 },
        ClassLevel { class_id: "fake-mage".to_string(), subclass_id: None, level: 3 },
    ];
    assert_eq!(total_level(&classes), 8);
}

#[test]
fn hp_uses_max_die_then_average() {
    assert_eq!(hp_max(12, 1, 2), 14);
    assert_eq!(hp_max(12, 3, 2), 32);
    assert_eq!(hp_max(6, 5, -4), 5);
}

fn hit_dice(entries: &[(&str, u8)]) -> HashMap<String, u8> {
    entries.iter().map(|(id, die)| (id.to_string(), *die)).collect()
}

#[test]
fn resolved_hp_max_multiclass_dispatches_by_method() {
    // character() is a single level-5 class, so this matches the old
    // single-class behavior exactly.
    let dice = hit_dice(&[("fake-warrior", 12)]);
    let mut hero = character();
    hero.hp_method = HpMethod::Average;
    assert_eq!(resolved_hp_max_multiclass(&hero, &dice, 2), hp_max(12, 5, 2));

    hero.hp_method = HpMethod::Rolled;
    hero.hp_rolls = vec![6, 8, 3, 10];
    // level 1: 12+2=14; levels 2-5: (6+2)+(8+2)+(3+2)+(10+2) = 35; total 49.
    assert_eq!(resolved_hp_max_multiclass(&hero, &dice, 2), 49);

    hero.hp_method = HpMethod::Manual;
    hero.hp_manual = Some(999);
    assert_eq!(resolved_hp_max_multiclass(&hero, &dice, 2), 999);
}

#[test]
fn resolved_hp_max_multiclass_manual_falls_back_to_average_when_unset() {
    let dice = hit_dice(&[("fake-warrior", 12)]);
    let mut hero = character();
    hero.hp_method = HpMethod::Manual;
    hero.hp_manual = None;
    assert_eq!(resolved_hp_max_multiclass(&hero, &dice, 2), hp_max(12, 5, 2));
}

#[test]
fn resolved_hp_max_multiclass_rolled_floors_at_one_per_level() {
    let dice = hit_dice(&[("fake-warrior", 6)]);
    let mut hero = character();
    hero.classes = vec![ClassLevel { class_id: "fake-warrior".to_string(), subclass_id: None, level: 3 }];
    hero.hp_method = HpMethod::Rolled;
    hero.hp_rolls = vec![1, 1];
    assert_eq!(resolved_hp_max_multiclass(&hero, &dice, -10), 3);
}

#[test]
fn resolved_hp_max_multiclass_uses_each_level_s_own_class_hit_die() {
    // Level 1 (Fighter, d10) maxed + con; levels 2-5 (still Fighter, d10 avg
    // gain = 6+con) x4; levels 6-8 (Wizard, d6 avg gain = 4+con) x3.
    let dice = hit_dice(&[("fake-warrior", 10), ("fake-mage", 6)]);
    let mut hero = character();
    hero.classes = vec![
        ClassLevel { class_id: "fake-warrior".to_string(), subclass_id: None, level: 5 },
        ClassLevel { class_id: "fake-mage".to_string(), subclass_id: None, level: 3 },
    ];
    hero.hp_method = HpMethod::Average;
    let expected = (10 + 2) + 4 * (6 + 2) + 3 * (4 + 2);
    assert_eq!(resolved_hp_max_multiclass(&hero, &dice, 2), expected);
}

#[test]
fn resolved_hp_max_multiclass_is_zero_with_no_classes() {
    let mut hero = character();
    hero.classes = vec![];
    assert_eq!(resolved_hp_max_multiclass(&hero, &HashMap::new(), 2), 0);
}

#[test]
fn validate_checks_hp_method_inputs() {
    let mut rolled = character();
    rolled.hp_method = HpMethod::Rolled;
    rolled.hp_rolls = vec![1, 2, 3];
    assert!(rolled.validate().is_err(), "wrong roll count for level 5 should fail");

    rolled.hp_rolls = vec![1, 2, 3, 0];
    assert!(rolled.validate().is_err(), "a zero roll should fail");

    rolled.hp_rolls = vec![1, 2, 3, 4];
    assert!(rolled.validate().is_ok());

    let mut manual = character();
    manual.hp_method = HpMethod::Manual;
    manual.hp_manual = None;
    assert!(manual.validate().is_err());

    manual.hp_manual = Some(0);
    assert!(manual.validate().is_err());

    manual.hp_manual = Some(40);
    assert!(manual.validate().is_ok());
}

#[test]
fn final_abilities_apply_asi_increases() {
    let mut character = character();
    character.asi_choices = vec![
        Some(AsiChoice::Abilities {
            codes: vec!["str".to_string(), "str".to_string()],
        }),
        Some(AsiChoice::Abilities {
            codes: vec!["dex".to_string(), "con".to_string()],
        }),
        Some(AsiChoice::Feat {
            feat_id: "fake-brawler".to_string(),
        }),
        None,
    ];

    let finals = final_abilities(&character, None);
    assert_eq!(finals[0], ("str", 17));
    assert_eq!(finals[1], ("dex", 15));
    assert_eq!(finals[2], ("con", 14));
    assert_eq!(finals[3], ("int", 12));
}

#[test]
fn final_abilities_cap_at_20() {
    let mut character = character();
    character.abilities.strength = 19;
    character.asi_choices = vec![Some(AsiChoice::Abilities {
        codes: vec!["str".to_string(), "str".to_string()],
    })];

    assert_eq!(final_abilities(&character, None)[0], ("str", 20));
}

#[test]
fn species_ability_bonus_applies_fixed_grants() {
    let species = species_with_bonuses(vec![AbilityBonusGrant::Fixed {
        code: "dex".to_string(),
        amount: 2,
    }]);
    let scores = apply_species_ability_bonus(character().abilities, Some(&species), &[]);
    assert_eq!(scores.dexterity, 16);
    assert_eq!(scores.strength, 15);
}

#[test]
fn species_ability_bonus_allows_negative_amounts() {
    let species = species_with_bonuses(vec![AbilityBonusGrant::Fixed {
        code: "int".to_string(),
        amount: -2,
    }]);
    let scores = apply_species_ability_bonus(character().abilities, Some(&species), &[]);
    assert_eq!(scores.intelligence, 10);
}

#[test]
fn species_ability_bonus_clamps_at_one_and_twenty() {
    let mut base = character().abilities;
    base.intelligence = 1;
    let low = species_with_bonuses(vec![AbilityBonusGrant::Fixed { code: "int".to_string(), amount: -5 }]);
    assert_eq!(apply_species_ability_bonus(base, Some(&low), &[]).intelligence, 1);

    let mut base = character().abilities;
    base.strength = 19;
    let high = species_with_bonuses(vec![AbilityBonusGrant::Fixed { code: "str".to_string(), amount: 5 }]);
    assert_eq!(apply_species_ability_bonus(base, Some(&high), &[]).strength, 20);
}

#[test]
fn species_ability_bonus_only_applies_choose_to_selected_codes() {
    let species = species_with_bonuses(vec![AbilityBonusGrant::Choose {
        count: 2,
        amount: 1,
        from: vec!["str".to_string(), "dex".to_string(), "con".to_string()],
    }]);
    let scores = apply_species_ability_bonus(
        character().abilities,
        Some(&species),
        &["str".to_string(), "con".to_string()],
    );
    assert_eq!(scores.strength, 16);
    assert_eq!(scores.dexterity, 14);
    assert_eq!(scores.constitution, 14);
}

#[test]
fn species_ability_bonus_choose_ignores_a_pick_outside_from() {
    let species = species_with_bonuses(vec![AbilityBonusGrant::Choose {
        count: 1,
        amount: 1,
        from: vec!["str".to_string()],
    }]);
    // A stale/invalid pick (e.g. left over from a previous species) should
    // be ignored rather than incorrectly bumping an unrelated ability.
    let scores = apply_species_ability_bonus(character().abilities, Some(&species), &["cha".to_string()]);
    assert_eq!(scores.charisma, 8);
}

#[test]
fn species_ability_bonus_choose_with_empty_from_is_a_free_choice() {
    let species = species_with_bonuses(vec![AbilityBonusGrant::Choose {
        count: 1,
        amount: 1,
        from: vec![],
    }]);
    let scores = apply_species_ability_bonus(character().abilities, Some(&species), &["wis".to_string()]);
    assert_eq!(scores.wisdom, 11);
}

#[test]
fn final_abilities_applies_species_bonus_before_asi() {
    let mut character = character();
    character.abilities.strength = 19;
    character.asi_choices = vec![Some(AsiChoice::Abilities {
        codes: vec!["str".to_string(), "str".to_string()],
    })];
    let species =
        species_with_bonuses(vec![AbilityBonusGrant::Fixed { code: "str".to_string(), amount: 2 }]);

    // 19 + 2 (species) clamps to 20, then ASI's +1 has no further room.
    assert_eq!(final_abilities(&character, Some(&species))[0], ("str", 20));
}

#[test]
fn apply_ability_choice_bumps_gives_plus_one_each_or_plus_two_for_a_repeat() {
    let scores = apply_ability_choice_bumps(
        character().abilities,
        &["str".to_string(), "dex".to_string()],
    );
    assert_eq!(scores.strength, 16);
    assert_eq!(scores.dexterity, 15);

    let doubled = apply_ability_choice_bumps(character().abilities, &["str".to_string(), "str".to_string()]);
    assert_eq!(doubled.strength, 17);
}

#[test]
fn apply_ability_choice_bumps_clamps_at_twenty() {
    let mut base = character().abilities;
    base.strength = 19;
    let scores = apply_ability_choice_bumps(base, &["str".to_string(), "str".to_string()]);
    assert_eq!(scores.strength, 20);
}

#[test]
fn resolve_ability_bonus_species_mode_matches_apply_species_ability_bonus() {
    let species = species_with_bonuses(vec![AbilityBonusGrant::Fixed {
        code: "dex".to_string(),
        amount: 2,
    }]);
    let expected = apply_species_ability_bonus(character().abilities, Some(&species), &[]);
    let actual = resolve_ability_bonus(
        character().abilities,
        Some(&species),
        AbilityBonusSource::Species,
        &[],
        &[],
    );
    assert_eq!(actual, expected);
}

#[test]
fn apply_custom_species_bonus_gives_plus_two_then_plus_one() {
    let scores = apply_custom_species_bonus(
        character().abilities,
        &["str".to_string(), "dex".to_string()],
    );
    assert_eq!(scores.strength, 17);
    assert_eq!(scores.dexterity, 15);
}

#[test]
fn apply_custom_species_bonus_clamps_each_independently_at_twenty() {
    let mut base = character().abilities;
    base.strength = 19;
    base.dexterity = 20;
    let scores = apply_custom_species_bonus(base, &["str".to_string(), "dex".to_string()]);
    assert_eq!(scores.strength, 20);
    assert_eq!(scores.dexterity, 20);
}

#[test]
fn resolve_ability_bonus_custom_mode_ignores_species_grants() {
    // A species with its own +2 Dex grant shouldn't apply at all once the
    // player has opted into a custom split — only the custom picks count.
    let species = species_with_bonuses(vec![AbilityBonusGrant::Fixed {
        code: "dex".to_string(),
        amount: 2,
    }]);
    let scores = resolve_ability_bonus(
        character().abilities,
        Some(&species),
        AbilityBonusSource::Custom,
        &[],
        &["con".to_string(), "wis".to_string()],
    );
    assert_eq!(scores.dexterity, 14);
    assert_eq!(scores.constitution, 15);
    assert_eq!(scores.wisdom, 11);
}

#[test]
fn final_abilities_stacks_custom_species_bonus_with_asi() {
    let mut character = character();
    character.ability_bonus_source = AbilityBonusSource::Custom;
    character.custom_ability_bonus_choices = vec!["str".to_string(), "dex".to_string()];
    character.abilities.strength = 19;
    character.asi_choices = vec![Some(AsiChoice::Abilities {
        codes: vec!["str".to_string(), "str".to_string()],
    })];

    // 19 + 2 (custom) clamps to 20, then ASI's +2 has no further room.
    assert_eq!(final_abilities(&character, None)[0], ("str", 20));
}

#[test]
fn validate_rejects_custom_bonus_with_wrong_count_invalid_code_or_a_repeat() {
    let mut character = character();
    character.ability_bonus_source = AbilityBonusSource::Custom;

    character.custom_ability_bonus_choices = vec!["str".to_string()];
    assert!(character.validate().is_err());

    character.custom_ability_bonus_choices = vec!["str".to_string(), "nope".to_string()];
    assert!(character.validate().is_err());

    // The two abilities must differ — a repeated code isn't the same thing
    // as ASI's "same twice for +2" and isn't allowed here.
    character.custom_ability_bonus_choices = vec!["str".to_string(), "str".to_string()];
    assert!(character.validate().is_err());

    character.custom_ability_bonus_choices = vec!["str".to_string(), "dex".to_string()];
    assert!(character.validate().is_ok());
}

#[test]
fn validate_rejects_mixing_species_and_custom_choices() {
    let mut species_mode = character();
    species_mode.ability_bonus_source = AbilityBonusSource::Species;
    species_mode.custom_ability_bonus_choices = vec!["str".to_string(), "dex".to_string()];
    assert!(species_mode.validate().is_err());

    let mut custom_mode = character();
    custom_mode.ability_bonus_source = AbilityBonusSource::Custom;
    custom_mode.custom_ability_bonus_choices = vec!["str".to_string(), "dex".to_string()];
    custom_mode.species_ability_choices = vec!["wis".to_string()];
    assert!(custom_mode.validate().is_err());
}

#[test]
fn asi_levels_filters_and_dedups() {
    let features = vec![
        feature("Rage", 1),
        feature("Ability Score Improvement", 4),
        feature("Extra Attack", 5),
        feature("Ability Score Improvement", 8),
        feature("Ability Score Improvement", 4),
    ];
    assert_eq!(asi_levels(&features), vec![4, 8]);
}

#[test]
fn multiclass_asi_slots_gates_each_class_on_its_own_level() {
    // A level-5 Fighter (ASI at Fighter-level 4)/3 Wizard (ASI at
    // Wizard-level 4, not yet reached) gets exactly 1 ASI slot.
    let mut features_by_class = HashMap::new();
    features_by_class.insert(
        "fake-warrior".to_string(),
        vec![feature("Ability Score Improvement", 4)],
    );
    features_by_class.insert(
        "fake-mage".to_string(),
        vec![feature("Ability Score Improvement", 4)],
    );
    let classes = vec![
        ClassLevel { class_id: "fake-warrior".to_string(), subclass_id: None, level: 5 },
        ClassLevel { class_id: "fake-mage".to_string(), subclass_id: None, level: 3 },
    ];
    assert_eq!(multiclass_asi_slots(&classes, &features_by_class), 1);

    let classes_both_unlocked = vec![
        ClassLevel { class_id: "fake-warrior".to_string(), subclass_id: None, level: 4 },
        ClassLevel { class_id: "fake-mage".to_string(), subclass_id: None, level: 4 },
    ];
    assert_eq!(multiclass_asi_slots(&classes_both_unlocked, &features_by_class), 2);
}

#[test]
fn multiclass_expertise_slots_sums_per_class() {
    let mut features_by_class = HashMap::new();
    features_by_class.insert("fake-rogue".to_string(), vec![feature("Expertise", 1), feature("Expertise", 6)]);
    features_by_class.insert("fake-bard".to_string(), vec![feature("Expertise", 3)]);
    let classes = vec![
        ClassLevel { class_id: "fake-rogue".to_string(), subclass_id: None, level: 6 },
        ClassLevel { class_id: "fake-bard".to_string(), subclass_id: None, level: 3 },
    ];
    assert_eq!(multiclass_expertise_slots(&classes, &features_by_class), 6);
}

#[test]
fn subclass_unlock_is_the_earliest_subclass_feature() {
    let detail = ClassDetail {
        class: Class {
            id: "fake-warrior".to_string(),
            canonical_id: "Fake Warrior|TBK".to_string(),
            name: "Fake Warrior".to_string(),
            source: "TBK".to_string(),
            hit_die: 10,
            saving_throws: vec!["str".to_string(), "con".to_string()],
            subclass_title: "Fighting Style".to_string(),
            caster_progression: None,
            spellcasting_ability: None,
            spells_known_progression: None,
            cantrips_known_progression: None,
            proficiencies: Proficiencies::default(),
            multiclass_proficiencies: Proficiencies::default(),
            starting_equipment: vec![],
            table_groups: vec![],
            optional_feature_progressions: vec![],
            multiclass_ability_prerequisites: vec![],
        },
        features: vec![],
        subclasses: vec![SubclassDetail {
            subclass: Subclass {
                id: "fake-duelist".to_string(),
                class_id: "fake-warrior".to_string(),
                name: "Fake Duelist".to_string(),
                short_name: "Duelist".to_string(),
                source: "TBK".to_string(),
                caster_progression: None,
                spellcasting_ability: None,
                spells_known_progression: None,
                cantrips_known_progression: None,
                optional_feature_progressions: vec![],
            },
            features: vec![feature("Dueling Stance", 3), feature("Riposte", 7)],
        }],
    };
    assert_eq!(subclass_unlock_level(&detail), Some(3));

    let no_subclasses = ClassDetail {
        subclasses: vec![],
        ..detail
    };
    assert_eq!(subclass_unlock_level(&no_subclasses), None);
}

#[test]
fn active_subclass_is_none_before_the_unlock_level_even_with_a_matching_id() {
    let detail = ClassDetail {
        class: Class {
            id: "fake-warrior".to_string(),
            canonical_id: "Fake Warrior|TBK".to_string(),
            name: "Fake Warrior".to_string(),
            source: "TBK".to_string(),
            hit_die: 10,
            saving_throws: vec!["str".to_string(), "con".to_string()],
            subclass_title: "Fighting Style".to_string(),
            caster_progression: None,
            spellcasting_ability: None,
            spells_known_progression: None,
            cantrips_known_progression: None,
            proficiencies: Proficiencies::default(),
            multiclass_proficiencies: Proficiencies::default(),
            starting_equipment: vec![],
            table_groups: vec![],
            optional_feature_progressions: vec![],
            multiclass_ability_prerequisites: vec![],
        },
        features: vec![],
        subclasses: vec![SubclassDetail {
            subclass: Subclass {
                id: "fake-duelist".to_string(),
                class_id: "fake-warrior".to_string(),
                name: "Fake Duelist".to_string(),
                short_name: "Duelist".to_string(),
                source: "TBK".to_string(),
                caster_progression: None,
                spellcasting_ability: None,
                spells_known_progression: None,
                cantrips_known_progression: None,
                optional_feature_progressions: vec![],
            },
            features: vec![feature("Dueling Stance", 3), feature("Riposte", 7)],
        }],
    };

    // A direct save_character POST (or any other stale/tampered state) could
    // carry a subclass_id that names a real subclass despite the character's
    // level being below its unlock — that must still resolve to None, not
    // the subclass, since the wizard's own UI would never offer it this early.
    assert_eq!(active_subclass(&detail, Some("fake-duelist"), 2), None);
    assert!(active_subclass(&detail, Some("fake-duelist"), 3).is_some());
    assert_eq!(active_subclass(&detail, Some("fake-duelist"), 3).unwrap().id, "fake-duelist");
    // No subclass_id at all is still None, regardless of level.
    assert_eq!(active_subclass(&detail, None, 10), None);
    // An unrecognized id at an unlocked level resolves to None too (no panic).
    assert_eq!(active_subclass(&detail, Some("not-a-real-id"), 10), None);
}

#[test]
fn validate_requires_the_core_choices() {
    assert!(character().validate().is_ok());

    let mut unnamed = character();
    unnamed.name = "  ".to_string();
    assert!(unnamed.validate().is_err());

    let mut no_class = character();
    no_class.classes = vec![];
    assert!(no_class.validate().is_err());

    let mut level_zero = character();
    level_zero.classes = vec![ClassLevel { class_id: "fake-warrior".to_string(), subclass_id: None, level: 0 }];
    assert!(level_zero.validate().is_err());
}

#[test]
fn validate_rejects_duplicate_classes() {
    let mut hero = character();
    hero.classes = vec![
        ClassLevel { class_id: "fake-warrior".to_string(), subclass_id: None, level: 3 },
        ClassLevel { class_id: "fake-warrior".to_string(), subclass_id: None, level: 2 },
    ];
    assert!(validate_err_contains(&hero, "more than once"));
}

#[test]
fn validate_accepts_multiple_distinct_classes() {
    let mut hero = character();
    hero.classes = vec![
        ClassLevel { class_id: "fake-warrior".to_string(), subclass_id: None, level: 5 },
        ClassLevel { class_id: "fake-mage".to_string(), subclass_id: None, level: 3 },
    ];
    hero.hp_rolls = vec![]; // Average method, no rolls needed.
    assert!(hero.validate().is_ok());
}

#[test]
fn validate_rejects_out_of_range_ability_scores() {
    let mut too_high = character();
    too_high.abilities.strength = 255;
    assert!(too_high.validate().is_err());

    let mut too_low = character();
    too_low.abilities.constitution = 0;
    assert!(too_low.validate().is_err());
}

#[test]
fn validate_rejects_malformed_asi_choices() {
    let mut empty_feat = character();
    empty_feat.asi_choices = vec![Some(AsiChoice::Feat { feat_id: String::new() })];
    assert!(empty_feat.validate().is_err());

    let mut bad_code = character();
    bad_code.asi_choices = vec![Some(AsiChoice::Abilities {
        codes: vec!["str".to_string(), "not-a-code".to_string()],
    })];
    assert!(bad_code.validate().is_err());

    let mut wrong_count = character();
    wrong_count.asi_choices = vec![Some(AsiChoice::Abilities { codes: vec!["str".to_string()] })];
    assert!(wrong_count.validate().is_err());

    let mut valid = character();
    valid.asi_choices = vec![
        Some(AsiChoice::Abilities { codes: vec!["str".to_string(), "str".to_string()] }),
        Some(AsiChoice::Feat { feat_id: "fake-brawler".to_string() }),
        None,
    ];
    assert!(valid.validate().is_ok());
}

#[test]
fn validate_rejects_invalid_or_duplicate_skill_choices() {
    let mut unknown_skill = character();
    unknown_skill.skill_choices = vec!["lockpicking".to_string()];
    assert!(unknown_skill.validate().is_err());

    let mut duplicate_choice = character();
    duplicate_choice.skill_choices = vec!["athletics".to_string(), "athletics".to_string()];
    assert!(duplicate_choice.validate().is_err());

    let mut duplicate_expertise = character();
    duplicate_expertise.expertise_choices = vec!["athletics".to_string(), "athletics".to_string()];
    assert!(duplicate_expertise.validate().is_err());

    let mut valid = character();
    valid.skill_choices = vec!["athletics".to_string(), "intimidation".to_string()];
    valid.expertise_choices = vec!["athletics".to_string()];
    assert!(valid.validate().is_ok());
}

#[test]
fn validate_rejects_invalid_or_duplicate_custom_proficiencies() {
    let mut unknown_skill = character();
    unknown_skill.custom_skill_proficiencies = vec!["lockpicking".to_string()];
    assert!(unknown_skill.validate().is_err());

    let mut duplicate_skill = character();
    duplicate_skill.custom_skill_proficiencies = vec!["athletics".to_string(), "athletics".to_string()];
    assert!(duplicate_skill.validate().is_err());

    let mut duplicate_language = character();
    duplicate_language.custom_language_proficiencies = vec!["draconic".to_string(), "draconic".to_string()];
    assert!(duplicate_language.validate().is_err());

    let mut duplicate_tool = character();
    duplicate_tool.custom_tool_proficiencies =
        vec!["thieves' tools".to_string(), "thieves' tools".to_string()];
    assert!(duplicate_tool.validate().is_err());

    let mut valid = character();
    valid.custom_skill_proficiencies = vec!["nature".to_string()];
    valid.custom_language_proficiencies = vec!["draconic".to_string()];
    valid.custom_tool_proficiencies = vec!["thieves' tools".to_string()];
    assert!(valid.validate().is_ok());
}

#[test]
fn skill_slots_dedups_fixed_grants_across_sources() {
    let class_skills = vec![("Fake Class".to_string(), SkillGrant::Fixed { skills: vec!["history".to_string()] })];
    let background_skills = vec![(
        "Fake Background".to_string(),
        SkillGrant::Fixed { skills: vec!["history".to_string(), "insight".to_string()] },
    )];
    let slots = skill_slots(&class_skills, &background_skills);
    assert_eq!(slots.fixed, vec!["history".to_string(), "insight".to_string()]);
    assert!(slots.choice_pools.is_empty());
}

#[test]
fn skill_slots_choose_grant_uses_its_own_list_when_uncontested() {
    let class_skills = vec![(
        "Fake Class".to_string(),
        SkillGrant::Choose {
            count: 2,
            from: vec!["arcana".to_string(), "deception".to_string(), "history".to_string()],
        },
    )];
    let slots = skill_slots(&class_skills, &[]);
    assert_eq!(slots.choice_pools.len(), 2);
    for pool in &slots.choice_pools {
        assert_eq!(pool.source, "Fake Class");
        assert_eq!(pool.options, vec!["arcana".to_string(), "deception".to_string(), "history".to_string()]);
    }
}

#[test]
fn skill_slots_any_grant_offers_the_full_skill_list() {
    let slots = skill_slots(&[("Fake Class".to_string(), SkillGrant::Any { count: 1 })], &[]);
    assert_eq!(slots.choice_pools.len(), 1);
    assert_eq!(slots.choice_pools[0].options.len(), SKILLS.len());
}

#[test]
fn skill_slots_falls_back_to_full_list_when_choices_collide_with_fixed_grants() {
    // Mirrors the fake-warrior/fake-wanderer e2e fixture collision: a class
    // offers "choose 2 from athletics/intimidation/survival" but the
    // background already fixed-grants two of those three.
    let class_skills = vec![(
        "Fake Class".to_string(),
        SkillGrant::Choose {
            count: 2,
            from: vec!["athletics".to_string(), "intimidation".to_string(), "survival".to_string()],
        },
    )];
    let background_skills = vec![(
        "Fake Background".to_string(),
        SkillGrant::Fixed { skills: vec!["athletics".to_string(), "survival".to_string()] },
    )];
    let slots = skill_slots(&class_skills, &background_skills);
    assert_eq!(slots.fixed, vec!["athletics".to_string(), "survival".to_string()]);
    assert_eq!(slots.choice_pools.len(), 2);
    for pool in &slots.choice_pools {
        assert_eq!(pool.options.len(), SKILLS.len(), "expected a fallback to the full skill list");
    }
}

#[test]
fn skill_slots_keeps_narrow_list_when_only_one_of_several_options_collides() {
    // arcana+deception alone already satisfy a required count of 2, so the
    // history/insight collision shouldn't trigger the full-list fallback.
    let class_skills = vec![(
        "Fake Class".to_string(),
        SkillGrant::Choose {
            count: 2,
            from: vec!["arcana".to_string(), "deception".to_string(), "history".to_string()],
        },
    )];
    let background_skills = vec![(
        "Fake Background".to_string(),
        SkillGrant::Fixed { skills: vec!["history".to_string(), "insight".to_string()] },
    )];
    let slots = skill_slots(&class_skills, &background_skills);
    assert_eq!(slots.choice_pools.len(), 2);
    for pool in &slots.choice_pools {
        assert_eq!(pool.options, vec!["arcana".to_string(), "deception".to_string(), "history".to_string()]);
    }
}

#[test]
fn language_slots_dedups_fixed_grants_across_species_and_background() {
    let background_languages =
        vec![("Fake Background".to_string(), LanguageGrant::Fixed { languages: vec!["common".to_string()] })];
    let species_languages = vec![(
        "Fake Species".to_string(),
        LanguageGrant::Fixed { languages: vec!["common".to_string(), "auran".to_string()] },
    )];
    let slots = language_slots(&background_languages, &species_languages, &[]);
    assert_eq!(slots.fixed, vec!["auran".to_string(), "common".to_string()]);
    assert!(slots.choice_pools.is_empty());
}

#[test]
fn language_slots_choose_grant_falls_back_to_all_languages_when_exhausted_by_fixed() {
    let all_languages =
        vec!["auran".to_string(), "aquan".to_string(), "common".to_string(), "draconic".to_string()];
    let background_languages = vec![(
        "Fake Background".to_string(),
        LanguageGrant::Choose { count: 1, from: vec!["auran".to_string()] },
    )];
    let species_languages =
        vec![("Fake Species".to_string(), LanguageGrant::Fixed { languages: vec!["auran".to_string()] })];
    let slots = language_slots(&background_languages, &species_languages, &all_languages);
    assert_eq!(slots.choice_pools.len(), 1);
    assert_eq!(slots.choice_pools[0].options, all_languages);
}

#[test]
fn language_slots_any_grant_offers_the_full_language_list() {
    let all_languages = vec!["auran".to_string(), "common".to_string()];
    let background_languages = vec![("Fake Background".to_string(), LanguageGrant::Any { count: 2 })];
    let slots = language_slots(&background_languages, &[], &all_languages);
    assert_eq!(slots.choice_pools.len(), 2);
    for pool in &slots.choice_pools {
        assert_eq!(pool.options, all_languages);
    }
}

#[test]
fn tool_slots_dedups_fixed_grants_across_class_and_background() {
    let class_tools =
        vec![("Fake Class".to_string(), ToolGrant::Fixed { tools: vec!["thieves' tools".to_string()] })];
    let background_tools = vec![(
        "Fake Background".to_string(),
        ToolGrant::Fixed { tools: vec!["thieves' tools".to_string(), "disguise kit".to_string()] },
    )];
    let slots = tool_slots(&class_tools, &background_tools, &HashMap::new());
    assert_eq!(slots.fixed, vec!["disguise kit".to_string(), "thieves' tools".to_string()]);
    assert!(slots.choice_pools.is_empty());
}

#[test]
fn tool_slots_any_category_grant_expands_against_category_members() {
    let mut category_members = HashMap::new();
    category_members.insert(
        ToolCategory::ArtisansTool,
        vec!["Alchemist's supplies".to_string(), "Brewer's supplies".to_string()],
    );
    let class_tools = vec![(
        "Fake Class".to_string(),
        ToolGrant::AnyCategory { count: 1, category: ToolCategory::ArtisansTool },
    )];
    let slots = tool_slots(&class_tools, &[], &category_members);
    assert_eq!(slots.choice_pools.len(), 1);
    assert_eq!(
        slots.choice_pools[0].options,
        vec!["Alchemist's supplies".to_string(), "Brewer's supplies".to_string()]
    );
}

#[test]
fn tool_slots_choose_grant_expands_mixed_named_and_category_options() {
    let mut category_members = HashMap::new();
    category_members.insert(ToolCategory::MusicalInstrument, vec!["Lute".to_string(), "Flute".to_string()]);
    let background_tools = vec![(
        "Fake Background".to_string(),
        ToolGrant::Choose {
            count: 1,
            from: vec![
                ToolOption::Category(ToolCategory::MusicalInstrument),
                ToolOption::Named("gaming set".to_string()),
            ],
        },
    )];
    let slots = tool_slots(&[], &background_tools, &category_members);
    assert_eq!(slots.choice_pools.len(), 1);
    assert_eq!(
        slots.choice_pools[0].options,
        vec!["Lute".to_string(), "Flute".to_string(), "gaming set".to_string()]
    );
}

#[test]
fn tool_slots_choose_availability_ignores_casing_against_a_fixed_grant() {
    // Real shape: a background fixed-grants "alchemist's supplies" (raw
    // import key, lowercase) while a class separately offers a category
    // choice whose DB-cased options include "Alchemist's supplies" — same
    // tool, different casing. A naive case-sensitive comparison would count
    // both category members as "not yet covered", satisfy count=2, and keep
    // the narrow 2-item list; recognizing the overlap leaves only 1 member
    // actually available, which isn't enough, so it must fall back to every
    // category's members instead (including "Dice Set", from a category the
    // Choose grant doesn't even list).
    let mut category_members = HashMap::new();
    category_members.insert(
        ToolCategory::ArtisansTool,
        vec!["Alchemist's supplies".to_string(), "Brewer's supplies".to_string()],
    );
    category_members.insert(ToolCategory::GamingSet, vec!["Dice Set".to_string()]);
    let class_tools = vec![(
        "Fake Class".to_string(),
        ToolGrant::Choose { count: 2, from: vec![ToolOption::Category(ToolCategory::ArtisansTool)] },
    )];
    let background_tools =
        vec![("Fake Background".to_string(), ToolGrant::Fixed { tools: vec!["alchemist's supplies".to_string()] })];
    let slots = tool_slots(&class_tools, &background_tools, &category_members);
    assert_eq!(slots.choice_pools.len(), 2);
    for pool in &slots.choice_pools {
        assert_eq!(
            pool.options,
            vec!["Alchemist's supplies".to_string(), "Brewer's supplies".to_string(), "Dice Set".to_string()],
            "expected a fallback to every category's members, not just the Choose grant's own ArtisansTool list"
        );
    }
}

#[test]
fn expertise_slots_counts_two_per_expertise_feature_unlocked() {
    let features = vec![
        feature("Sneak Attack", 1),
        feature("Expertise", 1),
        feature("Expertise", 6),
    ];
    assert_eq!(expertise_slots(&features, 1), 2);
    assert_eq!(expertise_slots(&features, 5), 2);
    assert_eq!(expertise_slots(&features, 6), 4);
}

fn caster_class(
    caster_progression: Option<&str>,
    spellcasting_ability: Option<&str>,
    spells_known_progression: Option<Vec<u8>>,
    cantrips_known_progression: Option<Vec<u8>>,
) -> Class {
    Class {
        id: "fake-caster".to_string(),
        canonical_id: "Fake Caster|TBK".to_string(),
        name: "Fake Caster".to_string(),
        source: "TBK".to_string(),
        hit_die: 6,
        saving_throws: vec![],
        subclass_title: "Fake School".to_string(),
        caster_progression: caster_progression.map(str::to_string),
        spellcasting_ability: spellcasting_ability.map(str::to_string),
        spells_known_progression,
        cantrips_known_progression,
        proficiencies: Proficiencies::default(),
        multiclass_proficiencies: Proficiencies::default(),
        starting_equipment: vec![],
        table_groups: vec![],
        optional_feature_progressions: vec![],
        multiclass_ability_prerequisites: vec![],
    }
}

fn class_with_prereqs(id: &str, name: &str, prereqs: Vec<Vec<(&str, u8)>>) -> Class {
    Class {
        id: id.to_string(),
        name: name.to_string(),
        multiclass_ability_prerequisites: prereqs
            .into_iter()
            .map(|group| group.into_iter().map(|(ability, min)| (ability.to_string(), min)).collect())
            .collect(),
        ..caster_class(None, None, None, None)
    }
}

fn classes_map(classes: Vec<Class>) -> HashMap<String, Class> {
    classes.into_iter().map(|c| (c.id.clone(), c)).collect()
}

#[test]
fn multiclass_prereq_violations_ignores_a_single_class_regardless_of_scores() {
    let mut hero = character();
    hero.abilities = AbilityScores {
        strength: 8,
        dexterity: 8,
        constitution: 8,
        intelligence: 8,
        wisdom: 8,
        charisma: 8,
    };
    hero.classes = vec![ClassLevel { class_id: "fake-warrior".to_string(), subclass_id: None, level: 5 }];
    let classes = classes_map(vec![class_with_prereqs("fake-warrior", "Warrior", vec![vec![("str", 13)]])]);
    assert!(multiclass_prereq_violations(&hero, &classes, None).is_empty());
}

#[test]
fn multiclass_prereq_violations_passes_when_every_class_meets_its_own_requirement() {
    let mut hero = character();
    hero.abilities = AbilityScores {
        strength: 13,
        dexterity: 10,
        constitution: 10,
        intelligence: 13,
        wisdom: 10,
        charisma: 10,
    };
    hero.classes = vec![
        ClassLevel { class_id: "fake-warrior".to_string(), subclass_id: None, level: 5 },
        ClassLevel { class_id: "fake-mage".to_string(), subclass_id: None, level: 1 },
    ];
    let classes = classes_map(vec![
        class_with_prereqs("fake-warrior", "Warrior", vec![vec![("str", 13)]]),
        class_with_prereqs("fake-mage", "Mage", vec![vec![("int", 13)]]),
    ]);
    assert!(multiclass_prereq_violations(&hero, &classes, None).is_empty());
}

#[test]
fn multiclass_prereq_violations_fighter_style_or_passes_with_either_ability() {
    let mut hero = character();
    hero.abilities = AbilityScores {
        strength: 8,
        dexterity: 13,
        constitution: 10,
        intelligence: 10,
        wisdom: 10,
        charisma: 13,
    };
    hero.classes = vec![
        ClassLevel { class_id: "fake-fighter".to_string(), subclass_id: None, level: 5 },
        ClassLevel { class_id: "fake-warlock".to_string(), subclass_id: None, level: 1 },
    ];
    let classes = classes_map(vec![
        class_with_prereqs("fake-fighter", "Fighter", vec![vec![("str", 13)], vec![("dex", 13)]]),
        class_with_prereqs("fake-warlock", "Warlock", vec![vec![("cha", 13)]]),
    ]);
    assert!(
        multiclass_prereq_violations(&hero, &classes, None).is_empty(),
        "Dex 13 alone should satisfy Fighter's Str-or-Dex requirement"
    );
}

#[test]
fn multiclass_prereq_violations_fighter_style_or_fails_when_neither_ability_met() {
    let mut hero = character();
    hero.abilities = AbilityScores {
        strength: 10,
        dexterity: 10,
        constitution: 10,
        intelligence: 10,
        wisdom: 10,
        charisma: 13,
    };
    hero.classes = vec![
        ClassLevel { class_id: "fake-fighter".to_string(), subclass_id: None, level: 5 },
        ClassLevel { class_id: "fake-warlock".to_string(), subclass_id: None, level: 1 },
    ];
    let classes = classes_map(vec![
        class_with_prereqs("fake-fighter", "Fighter", vec![vec![("str", 13)], vec![("dex", 13)]]),
        class_with_prereqs("fake-warlock", "Warlock", vec![vec![("cha", 13)]]),
    ]);
    let violations = multiclass_prereq_violations(&hero, &classes, None);
    assert_eq!(violations.len(), 1);
    assert!(violations[0].contains("Fighter"));
}

#[test]
fn multiclass_prereq_violations_and_group_fails_with_only_one_ability_met() {
    // Str 13 AND Cha 13 both required, not either alone.
    let mut hero = character();
    hero.abilities = AbilityScores {
        strength: 13,
        dexterity: 10,
        constitution: 10,
        intelligence: 10,
        wisdom: 10,
        charisma: 8,
    };
    hero.classes = vec![
        ClassLevel { class_id: "fake-paladin".to_string(), subclass_id: None, level: 5 },
        ClassLevel { class_id: "fake-warrior".to_string(), subclass_id: None, level: 1 },
    ];
    let classes = classes_map(vec![
        class_with_prereqs("fake-paladin", "Paladin", vec![vec![("str", 13), ("cha", 13)]]),
        class_with_prereqs("fake-warrior", "Warrior", vec![vec![("str", 13)]]),
    ]);
    let violations = multiclass_prereq_violations(&hero, &classes, None);
    assert_eq!(violations.len(), 1);
    assert!(violations[0].contains("Paladin"));
}

#[test]
fn multiclass_prereq_violations_are_suppressed_when_enforcement_is_off() {
    let mut hero = character();
    hero.enforce_multiclass_prereqs = false;
    hero.abilities = AbilityScores {
        strength: 8,
        dexterity: 8,
        constitution: 8,
        intelligence: 8,
        wisdom: 8,
        charisma: 8,
    };
    hero.classes = vec![
        ClassLevel { class_id: "fake-fighter".to_string(), subclass_id: None, level: 5 },
        ClassLevel { class_id: "fake-warlock".to_string(), subclass_id: None, level: 1 },
    ];
    let classes = classes_map(vec![
        class_with_prereqs("fake-fighter", "Fighter", vec![vec![("str", 13)], vec![("dex", 13)]]),
        class_with_prereqs("fake-warlock", "Warlock", vec![vec![("cha", 13)]]),
    ]);
    assert!(multiclass_prereq_violations(&hero, &classes, None).is_empty());
}

#[test]
fn multiclass_prereq_violations_never_flags_a_class_with_no_prerequisites_set() {
    let mut hero = character();
    hero.abilities = AbilityScores {
        strength: 8,
        dexterity: 8,
        constitution: 8,
        intelligence: 8,
        wisdom: 8,
        charisma: 8,
    };
    hero.classes = vec![
        ClassLevel { class_id: "fake-warrior".to_string(), subclass_id: None, level: 5 },
        ClassLevel { class_id: "fake-mage".to_string(), subclass_id: None, level: 1 },
    ];
    // Neither class has prerequisites set.
    let classes = classes_map(vec![
        class_with_prereqs("fake-warrior", "Warrior", vec![]),
        class_with_prereqs("fake-mage", "Mage", vec![]),
    ]);
    assert!(multiclass_prereq_violations(&hero, &classes, None).is_empty());
}

#[test]
fn multiclass_prereq_violations_uses_final_abilities_including_species_bonus() {
    // A +1 racial bonus should push Int 12 over Mage's Int 13 requirement.
    let mut hero = character();
    hero.abilities = AbilityScores {
        strength: 13,
        dexterity: 10,
        constitution: 10,
        intelligence: 12,
        wisdom: 10,
        charisma: 10,
    };
    hero.classes = vec![
        ClassLevel { class_id: "fake-warrior".to_string(), subclass_id: None, level: 5 },
        ClassLevel { class_id: "fake-mage".to_string(), subclass_id: None, level: 1 },
    ];
    let classes = classes_map(vec![
        class_with_prereqs("fake-warrior", "Warrior", vec![vec![("str", 13)]]),
        class_with_prereqs("fake-mage", "Mage", vec![vec![("int", 13)]]),
    ]);
    let species = species_with_bonuses(vec![AbilityBonusGrant::Fixed { code: "int".to_string(), amount: 1 }]);

    assert_eq!(
        multiclass_prereq_violations(&hero, &classes, None).len(),
        1,
        "without species bonus, base Int 12 should fail"
    );
    assert!(
        multiclass_prereq_violations(&hero, &classes, Some(&species)).is_empty(),
        "with the +1 racial bonus, final Int 13 should pass"
    );
}

#[test]
fn spell_slots_match_srd_full_caster_table_at_key_levels() {
    assert_eq!(spell_slots_for_level("full", 1), [2, 0, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(spell_slots_for_level("full", 5), [4, 3, 2, 0, 0, 0, 0, 0, 0]);
    assert_eq!(spell_slots_for_level("full", 17), [4, 3, 3, 3, 2, 1, 1, 1, 1]);
    assert_eq!(spell_slots_for_level("full", 20), [4, 3, 3, 3, 3, 2, 2, 1, 1]);
}

#[test]
fn spell_slots_match_srd_half_caster_table_at_key_levels() {
    assert_eq!(spell_slots_for_level("half", 1), [0; 9]);
    assert_eq!(spell_slots_for_level("half", 2), [2, 0, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(spell_slots_for_level("half", 5), [4, 2, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(spell_slots_for_level("half", 20), [4, 3, 3, 3, 2, 0, 0, 0, 0]);
}

#[test]
fn spell_slots_match_the_third_caster_table_at_key_levels() {
    assert_eq!(spell_slots_for_level("third", 1), [0; 9]);
    assert_eq!(spell_slots_for_level("third", 3), [2, 0, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(spell_slots_for_level("third", 7), [4, 2, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(spell_slots_for_level("third", 19), [4, 3, 3, 1, 0, 0, 0, 0, 0]);
}

#[test]
fn spell_slots_match_the_pact_magic_table_at_key_levels() {
    // Pact slots all share one spell level, unlike full/half/third casters.
    assert_eq!(spell_slots_for_level("pact", 1), [1, 0, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(spell_slots_for_level("pact", 3), [0, 2, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(spell_slots_for_level("pact", 11), [0, 0, 0, 0, 3, 0, 0, 0, 0]);
    assert_eq!(spell_slots_for_level("pact", 20), [0, 0, 0, 0, 4, 0, 0, 0, 0]);
}

#[test]
fn spell_slots_match_the_artificer_table_at_key_levels() {
    // Level 1 borrows the half-caster's level-2 row, then tracks it exactly.
    assert_eq!(spell_slots_for_level("artificer", 1), [2, 0, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(spell_slots_for_level("artificer", 2), [2, 0, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(spell_slots_for_level("artificer", 5), spell_slots_for_level("half", 5));
    assert_eq!(spell_slots_for_level("artificer", 20), spell_slots_for_level("half", 20));
}

#[test]
fn spell_slots_are_zero_for_unmodeled_progressions() {
    assert_eq!(spell_slots_for_level("", 5), [0; 9]);
    assert_eq!(spell_slots_for_level("unknown", 5), [0; 9]);
}

#[test]
fn max_castable_spell_level_is_the_highest_nonzero_slot() {
    assert_eq!(max_castable_spell_level("full", 1), 1);
    assert_eq!(max_castable_spell_level("full", 3), 2);
    assert_eq!(max_castable_spell_level("full", 20), 9);
    assert_eq!(max_castable_spell_level("half", 1), 0);
    assert_eq!(max_castable_spell_level("half", 20), 5);
    assert_eq!(max_castable_spell_level("pact", 1), 1);
    assert_eq!(max_castable_spell_level("pact", 20), 5);
    assert_eq!(max_castable_spell_level("third", 1), 0);
    assert_eq!(max_castable_spell_level("third", 19), 4);
    assert_eq!(max_castable_spell_level("artificer", 1), 1);
}

fn progressions(entries: &[(&str, &str)]) -> HashMap<String, Option<String>> {
    entries.iter().map(|(id, p)| (id.to_string(), Some(p.to_string()))).collect()
}

#[test]
fn multiclass_caster_level_sums_full_half_and_third_casters() {
    // The canonical 5e example: a 3rd-level Wizard (full) / 3rd-level Cleric
    // (full) is caster level 6.
    let classes = vec![
        ClassLevel { class_id: "fake-wizard".to_string(), subclass_id: None, level: 3 },
        ClassLevel { class_id: "fake-cleric".to_string(), subclass_id: None, level: 3 },
    ];
    let progs = progressions(&[("fake-wizard", "full"), ("fake-cleric", "full")]);
    assert_eq!(multiclass_caster_level(&classes, &progs), 6);
    assert_eq!(multiclass_spell_slots(&classes, &progs), spell_slots_for_level("full", 6));

    // A level-6 Paladin (half, contributes 3) / level-4 Fighter-with-no-caster-progression
    // (contributes 0) is caster level 3.
    let mixed = vec![
        ClassLevel { class_id: "fake-paladin".to_string(), subclass_id: None, level: 6 },
        ClassLevel { class_id: "fake-warrior".to_string(), subclass_id: None, level: 4 },
    ];
    let mixed_progs = progressions(&[("fake-paladin", "half")]);
    assert_eq!(multiclass_caster_level(&mixed, &mixed_progs), 3);
}

#[test]
fn multiclass_spell_slots_uses_the_solo_class_s_own_table_for_a_single_half_or_third_caster() {
    // A solo half-caster (Paladin/Ranger-shaped) at level 5 has slots [4,2]
    // per its own printed table — NOT [3,0], which is what floor(5/2)=2
    // looked up in the full-caster table would (wrongly) give. The combined
    // multiclass formula only kicks in with 2+ distinct caster classes.
    let solo_half = vec![ClassLevel { class_id: "fake-paladin".to_string(), subclass_id: None, level: 5 }];
    let half_progs = progressions(&[("fake-paladin", "half")]);
    assert_eq!(multiclass_spell_slots(&solo_half, &half_progs), spell_slots_for_level("half", 5));
    assert_ne!(multiclass_spell_slots(&solo_half, &half_progs), spell_slots_for_level("full", 2));

    // A solo third-caster (Eldritch Knight-shaped) likewise uses its own table.
    let solo_third = vec![ClassLevel { class_id: "fake-eldritch-knight".to_string(), subclass_id: None, level: 7 }];
    let third_progs = progressions(&[("fake-eldritch-knight", "third")]);
    assert_eq!(multiclass_spell_slots(&solo_third, &third_progs), spell_slots_for_level("third", 7));

    // A solo full caster is formula-identity-preserving either way.
    let solo_full = vec![ClassLevel { class_id: "fake-wizard".to_string(), subclass_id: None, level: 5 }];
    let full_progs = progressions(&[("fake-wizard", "full")]);
    assert_eq!(multiclass_spell_slots(&solo_full, &full_progs), spell_slots_for_level("full", 5));
}

#[test]
fn multiclass_pact_slots_never_merge_into_the_shared_pool() {
    // A Warlock mixed in contributes 0 to the shared full-caster pool, but
    // keeps its own separate Pact Magic slots.
    let classes = vec![
        ClassLevel { class_id: "fake-wizard".to_string(), subclass_id: None, level: 3 },
        ClassLevel { class_id: "fake-warlock".to_string(), subclass_id: None, level: 3 },
    ];
    let progs = progressions(&[("fake-wizard", "full"), ("fake-warlock", "pact")]);
    assert_eq!(multiclass_caster_level(&classes, &progs), 3);
    assert_eq!(multiclass_spell_slots(&classes, &progs), spell_slots_for_level("full", 3));
    assert_eq!(multiclass_pact_slots(&classes, &progs), spell_slots_for_level("pact", 3));
}

#[test]
fn cantrips_known_count_reads_the_progression_table() {
    let known = effective_spellcasting(&caster_class(Some("full"), Some("int"), None, Some(vec![3, 4, 5])), None);
    assert_eq!(cantrips_known_count(&known, 1), 3);
    assert_eq!(cantrips_known_count(&known, 3), 5);

    let no_cantrips = effective_spellcasting(&caster_class(Some("half"), Some("cha"), None, None), None);
    assert_eq!(cantrips_known_count(&no_cantrips, 5), 0);
}

#[test]
fn spells_known_count_uses_the_fixed_table_when_present() {
    let known = effective_spellcasting(&caster_class(Some("full"), Some("cha"), Some(vec![2, 3, 4]), None), None);
    // Fixed table wins regardless of ability score.
    assert_eq!(spells_known_count(&known, 1, 8), 2);
    assert_eq!(spells_known_count(&known, 3, 20), 4);
}

#[test]
fn spells_known_count_prepared_formula_uses_full_half_or_artificer_level_component() {
    let full_prepared = effective_spellcasting(&caster_class(Some("full"), Some("int"), None, None), None);
    // level + int_mod, int 16 => mod +3
    assert_eq!(spells_known_count(&full_prepared, 5, 16), 8);

    let half_prepared = effective_spellcasting(&caster_class(Some("half"), Some("cha"), None, None), None);
    // level/2 + cha_mod, level 9 => 4, cha 14 => mod +2
    assert_eq!(spells_known_count(&half_prepared, 9, 14), 6);

    let artificer_prepared = effective_spellcasting(&caster_class(Some("artificer"), Some("int"), None, None), None);
    // level/2 + int_mod, level 9 => 4, int 14 => mod +2
    assert_eq!(spells_known_count(&artificer_prepared, 9, 14), 6);
}

#[test]
fn spells_known_count_floors_at_one() {
    let full_prepared = effective_spellcasting(&caster_class(Some("full"), Some("int"), None, None), None);
    // level 1 + int_mod for int 8 (-1) => 0, floored to 1
    assert_eq!(spells_known_count(&full_prepared, 1, 8), 1);
}

#[test]
fn spells_known_count_is_zero_for_non_casters() {
    let non_caster = effective_spellcasting(&caster_class(None, None, None, None), None);
    assert_eq!(spells_known_count(&non_caster, 10, 15), 0);
}

#[test]
fn effective_spellcasting_prefers_class_fields_when_present() {
    let class = caster_class(Some("full"), Some("int"), None, Some(vec![3]));
    let subclass = Subclass {
        id: "fake-school".to_string(),
        class_id: class.id.clone(),
        name: "Fake School".to_string(),
        short_name: "Fakery".to_string(),
        source: "TBK".to_string(),
        caster_progression: Some("pact".to_string()),
        spellcasting_ability: Some("cha".to_string()),
        spells_known_progression: None,
        cantrips_known_progression: None,
        optional_feature_progressions: vec![],
    };
    let profile = effective_spellcasting(&class, Some(&subclass));
    assert_eq!(profile.caster_progression, Some("full".to_string()));
    assert_eq!(profile.spellcasting_ability, Some("int".to_string()));
}

#[test]
fn effective_spellcasting_falls_back_to_subclass_when_class_has_none() {
    let class = caster_class(None, None, None, None);
    let subclass = Subclass {
        id: "fake-eldritch-knight".to_string(),
        class_id: class.id.clone(),
        name: "Fake Eldritch Knight".to_string(),
        short_name: "Fake Eldritch Knight".to_string(),
        source: "TBK".to_string(),
        caster_progression: Some("third".to_string()),
        spellcasting_ability: Some("int".to_string()),
        spells_known_progression: Some(vec![0, 0, 3]),
        cantrips_known_progression: Some(vec![0, 0, 2]),
        optional_feature_progressions: vec![],
    };
    let profile = effective_spellcasting(&class, Some(&subclass));
    assert_eq!(profile.caster_progression, Some("third".to_string()));
    assert_eq!(profile.spellcasting_ability, Some("int".to_string()));
    assert_eq!(cantrips_known_count(&profile, 3), 2);
    assert_eq!(spells_known_count(&profile, 3, 10), 3);

    // A non-casting subclass never overrides a genuinely non-caster class.
    let non_caster_subclass = Subclass {
        caster_progression: None,
        ..subclass
    };
    let empty_profile = effective_spellcasting(&class, Some(&non_caster_subclass));
    assert_eq!(empty_profile, SpellcastingProfile::default());
}

#[test]
fn multiclass_spell_profiles_computes_each_class_against_its_own_level_and_ability() {
    let wizard = caster_class(Some("full"), Some("int"), None, Some(vec![3, 3, 3]));
    let cleric = Class { id: "fake-cleric".to_string(), ..caster_class(Some("full"), Some("wis"), None, None) };
    let mut lookup = HashMap::new();
    lookup.insert(wizard.id.clone(), (wizard.clone(), None));
    lookup.insert(cleric.id.clone(), (cleric.clone(), None));

    let classes = vec![
        ClassLevel { class_id: wizard.id.clone(), subclass_id: None, level: 3 },
        ClassLevel { class_id: cleric.id.clone(), subclass_id: None, level: 3 },
    ];
    let finals: [(&str, u8); 6] = [("str", 10), ("dex", 10), ("con", 10), ("int", 16), ("wis", 14), ("cha", 10)];
    let profiles = multiclass_spell_profiles(&classes, &lookup, &finals);

    assert_eq!(profiles.len(), 2);
    let wizard_profile = profiles.iter().find(|p| p.class_id == wizard.id).unwrap();
    assert_eq!(wizard_profile.known_cantrips, 3);
    // level 3 + int mod (+3) = 6 prepared.
    assert_eq!(wizard_profile.known_spells, 6);
    let cleric_profile = profiles.iter().find(|p| p.class_id == cleric.id).unwrap();
    // level 3 + wis mod (+2) = 5 prepared.
    assert_eq!(cleric_profile.known_spells, 5);
}

#[test]
fn spell_save_dc_and_attack_bonus_add_proficiency_and_modifier() {
    assert_eq!(spell_save_dc(3, 3), 14);
    assert_eq!(spell_attack_bonus(3, 3), 6);
    assert_eq!(spell_save_dc(2, -1), 9);
}

#[test]
fn optional_feature_quota_sums_class_and_subclass_progressions_of_the_same_type() {
    let class = Class {
        optional_feature_progressions: vec![OptionalFeatureProgression {
            feature_type: FeatureType::FightingStyleFighter,
            known: vec![1; 20], // known from level 1 onward
        }],
        ..caster_class(None, None, None, None)
    };
    let subclass = Subclass {
        // Champion-style: a second Fighting Style pick starting at level 10.
        optional_feature_progressions: vec![OptionalFeatureProgression {
            feature_type: FeatureType::FightingStyleFighter,
            known: (1..=20).map(|level| if level >= 10 { 1 } else { 0 }).collect(),
        }],
        ..subclass_with(&class, "Fake Champion")
    };

    assert_eq!(optional_feature_quota(&class, None, FeatureType::FightingStyleFighter, 5), 1);
    assert_eq!(optional_feature_quota(&class, Some(&subclass), FeatureType::FightingStyleFighter, 9), 1);
    assert_eq!(optional_feature_quota(&class, Some(&subclass), FeatureType::FightingStyleFighter, 10), 2);
    // A type neither the class nor subclass grants is always zero.
    assert_eq!(optional_feature_quota(&class, Some(&subclass), FeatureType::EldritchInvocation, 20), 0);
}

#[test]
fn active_optional_feature_types_excludes_zero_quota_types() {
    let class = Class {
        optional_feature_progressions: vec![
            OptionalFeatureProgression { feature_type: FeatureType::EldritchInvocation, known: vec![0, 2, 2] },
            OptionalFeatureProgression { feature_type: FeatureType::PactBoon, known: vec![0, 0, 1] },
        ],
        ..caster_class(None, None, None, None)
    };

    assert_eq!(active_optional_feature_types(&class, None, 1), Vec::<FeatureType>::new());
    assert_eq!(active_optional_feature_types(&class, None, 2), vec![FeatureType::EldritchInvocation]);
    let mut at_3 = active_optional_feature_types(&class, None, 3);
    at_3.sort_by_key(FeatureType::as_str);
    assert_eq!(at_3, vec![FeatureType::EldritchInvocation, FeatureType::PactBoon]);
}

#[test]
fn multiclass_optional_feature_quota_sums_across_classes() {
    let fighter = Class {
        id: "fake-warrior".to_string(),
        optional_feature_progressions: vec![OptionalFeatureProgression {
            feature_type: FeatureType::FightingStyleFighter,
            known: vec![1; 20],
        }],
        ..caster_class(None, None, None, None)
    };
    let warlock = Class {
        id: "fake-warlock".to_string(),
        optional_feature_progressions: vec![OptionalFeatureProgression {
            feature_type: FeatureType::EldritchInvocation,
            known: vec![2; 20],
        }],
        ..caster_class(Some("pact"), Some("cha"), None, None)
    };

    let entries: Vec<(&Class, Option<&Subclass>, u8)> = vec![(&fighter, None, 5), (&warlock, None, 3)];
    assert_eq!(multiclass_optional_feature_quota(&entries, FeatureType::FightingStyleFighter), 1);
    assert_eq!(multiclass_optional_feature_quota(&entries, FeatureType::EldritchInvocation), 2);

    let mut types = multiclass_active_optional_feature_types(&entries);
    types.sort_by_key(FeatureType::as_str);
    assert_eq!(types, vec![FeatureType::EldritchInvocation, FeatureType::FightingStyleFighter]);
}

#[test]
fn validate_rejects_duplicate_optional_feature_choices() {
    let mut hero = character();
    hero.optional_feature_choices = vec!["agonizing-blast".to_string(), "agonizing-blast".to_string()];
    assert!(validate_err_contains(&hero, "chosen more than once"));
}

fn subclass_with(class: &Class, name: &str) -> Subclass {
    Subclass {
        id: format!("{}-subclass", class.id),
        class_id: class.id.clone(),
        name: name.to_string(),
        short_name: name.to_string(),
        source: "TBK".to_string(),
        caster_progression: None,
        spellcasting_ability: None,
        spells_known_progression: None,
        cantrips_known_progression: None,
        optional_feature_progressions: vec![],
    }
}

fn validate_err_contains(character: &Character, needle: &str) -> bool {
    matches!(character.validate(), Err(msg) if msg.contains(needle))
}
