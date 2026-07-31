use crate::importer::entries::{clean_tags, entries_from_values, progression_cell_text};
use crate::importer::parse_class::{
    RawAdditionalSpells, RawClass, RawClassFile, RawMulticlassing, RawOptionalFeatureProgressionEntry,
    RawOptionalFeatureProgressionShape, RawStartingEquipment, RawStartingProficiencies, RawSubclass,
    RawTableGroup,
};
use crate::importer::transform::slugify;
use crate::models::class::{Class, ClassFeature, ClassTableGroup, OptionalFeatureProgression, Proficiencies, Subclass};
use crate::models::optional_feature::FeatureType;
use crate::models::skill::SkillGrant;
use serde_json::Value;

/// A class plus everything imported alongside it, ready to insert.
pub struct ClassBundle {
    pub class: Class,
    pub features: Vec<ClassFeature>,
    pub subclasses: Vec<(Subclass, Vec<ClassFeature>, Vec<GrantedSpellRef>)>,
}

/// One resolvable "this subclass auto-grants this spell at this level" grant,
/// prior to matching the spell name against the already-seeded `spells`
/// table (see `seed_classes` in `src/bin/seed.rs`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantedSpellRef {
    pub spell_name_lower: String,
    pub level: u8,
}

/// Extracts spell names from one level's `additionalSpells` value. Only a
/// flat JSON array of strings is a fixed grant; anything else (a
/// `{"choose": ...}` filter object, or the `{"daily": {...}}` wrapper seen on
/// a couple of Warlock patrons) is skipped — neither has a wizard-side choice
/// UI yet, tracked as a known gap in TDD.md's Phase 6 notes rather than left
/// only here. Names sometimes carry a `#c`/`|SOURCE` tag suffix
/// (`"light#c"`, `"fire shield|"`) that isn't part of the spell's actual name.
fn granted_spell_names_at_level(value: &Value) -> Vec<String> {
    let Some(items) = value.as_array() else { return Vec::new() };
    items
        .iter()
        .filter_map(Value::as_str)
        .filter_map(|name| {
            let clean = name.split(['#', '|']).next().unwrap_or(name).trim();
            (!clean.is_empty()).then(|| clean.to_lowercase())
        })
        .collect()
}

fn granted_spells_from_entry(entry: &RawAdditionalSpells) -> Vec<GrantedSpellRef> {
    let mut grants = Vec::new();
    for level_map in [&entry.prepared, &entry.known] {
        for (level_str, value) in level_map {
            let Ok(level) = level_str.parse::<u8>() else { continue };
            grants.extend(
                granted_spell_names_at_level(value)
                    .into_iter()
                    .map(|spell_name_lower| GrantedSpellRef { spell_name_lower, level }),
            );
        }
    }
    grants
}

/// Groups a subclass's `additionalSpells` entries by variant name (`None` =
/// ungrouped/common grants — the typical case is zero or one such group).
/// A subclass shaped like Druid Circle of the Land instead has several
/// *named* groups (one per terrain); `class_bundles_from_parsed` fans those
/// out into separate `Subclass` rows rather than modeling a player-facing
/// sub-choice.
fn granted_spell_groups(raw: &RawSubclass) -> Vec<(Option<String>, Vec<GrantedSpellRef>)> {
    let mut groups: Vec<(Option<String>, Vec<GrantedSpellRef>)> = Vec::new();
    for entry in &raw.additional_spells {
        let grants = granted_spells_from_entry(entry);
        match groups.iter_mut().find(|(name, _)| *name == entry.name) {
            Some((_, existing)) => existing.extend(grants),
            None => groups.push((entry.name.clone(), grants)),
        }
    }
    groups
}

pub fn class_bundles_from_parsed(file: RawClassFile) -> anyhow::Result<Vec<ClassBundle>> {
    let RawClassFile { class, subclass, class_feature, subclass_feature } = file;

    class
        .into_iter()
        .map(|raw| {
            let features = class_feature
                .iter()
                .filter(|f| f.class_name == raw.name)
                .map(|f| ClassFeature {
                    name: clean_tags(&f.name),
                    source: f.source.clone(),
                    level: f.level,
                    entries: entries_from_values(&f.entries),
                })
                .collect();

            let subclasses = subclass
                .iter()
                .filter(|s| s.class_name == raw.name)
                .flat_map(|s| {
                    let features: Vec<ClassFeature> = subclass_feature
                        .iter()
                        .filter(|f| {
                            f.class_name == s.class_name
                                && f.subclass_short_name == s.short_name
                                && f.subclass_source == s.source
                        })
                        .map(|f| ClassFeature {
                            name: clean_tags(&f.name),
                            source: f.source.clone(),
                            level: f.level,
                            entries: entries_from_values(&f.entries),
                        })
                        .collect();

                    let groups = granted_spell_groups(s);
                    let has_named_variant = groups.iter().any(|(name, _)| name.is_some());

                    if !has_named_variant {
                        let grants = groups.into_iter().next().map(|(_, g)| g).unwrap_or_default();
                        let subclass = Subclass {
                            id: slugify(&format!("{} {} {}", raw.name, s.short_name, s.source)),
                            class_id: slugify(&raw.name),
                            name: s.name.clone(),
                            short_name: s.short_name.clone(),
                            source: s.source.clone(),
                            caster_progression: normalize_caster_progression(s.caster_progression.clone()),
                            spellcasting_ability: s.spellcasting_ability.clone(),
                            spells_known_progression: s.spells_known_progression.clone(),
                            cantrips_known_progression: s.cantrips_known_progression.clone(),
                            optional_feature_progressions: optional_feature_progressions_from_raw(
                                &s.optional_feature_progression,
                            ),
                        };
                        vec![(subclass, features, grants)]
                    } else {
                        // Circle-of-the-Land shape: fan out into one Subclass
                        // per named variant, sharing `features` (the terrain
                        // doesn't change class features, only spell grants),
                        // each carrying any ungrouped/common grants plus its
                        // own variant's grants.
                        let common: Vec<GrantedSpellRef> = groups
                            .iter()
                            .find(|(name, _)| name.is_none())
                            .map(|(_, g)| g.clone())
                            .unwrap_or_default();
                        groups
                            .into_iter()
                            .filter_map(|(name, grants)| name.map(|name| (name, grants)))
                            .map(|(variant_name, grants)| {
                                let mut all_grants = common.clone();
                                all_grants.extend(grants);
                                let subclass = Subclass {
                                    id: slugify(&format!(
                                        "{} {} ({}) {}",
                                        raw.name, s.short_name, variant_name, s.source
                                    )),
                                    class_id: slugify(&raw.name),
                                    name: format!("{} ({})", s.name, variant_name),
                                    short_name: format!("{} ({})", s.short_name, variant_name),
                                    source: s.source.clone(),
                                    caster_progression: normalize_caster_progression(
                                        s.caster_progression.clone(),
                                    ),
                                    spellcasting_ability: s.spellcasting_ability.clone(),
                                    spells_known_progression: s.spells_known_progression.clone(),
                                    cantrips_known_progression: s.cantrips_known_progression.clone(),
                                    optional_feature_progressions: optional_feature_progressions_from_raw(
                                        &s.optional_feature_progression,
                                    ),
                                };
                                (subclass, features.clone(), all_grants)
                            })
                            .collect()
                    }
                })
                .collect();

            Ok(ClassBundle { class: class_from_raw(raw), features, subclasses })
        })
        .collect()
}

/// Normalizes a class/subclass's `optionalfeatureProgression` entries into
/// `OptionalFeatureProgression`s, dropping entries whose `featureType`
/// code(s) aren't recognized (tolerated, matching how
/// `transform_optional_feature.rs` treats unrecognized codes elsewhere —
/// not a hard import error).
fn optional_feature_progressions_from_raw(
    raw: &[RawOptionalFeatureProgressionEntry],
) -> Vec<OptionalFeatureProgression> {
    raw.iter()
        .flat_map(|entry| {
            let known = dense_progression(&entry.progression);
            entry
                .feature_type
                .iter()
                .filter_map(|code| FeatureType::from_code(code))
                .map(move |feature_type| OptionalFeatureProgression { feature_type, known: known.clone() })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Normalizes one `progression` value to the same dense, level-1-indexed
/// convention `spells_known_progression` already uses. The dense-array shape
/// passes through as-is. The sparse map shape (`{"3": 2, "10": 3}`) is
/// forward-filled: level keys (unordered JSON-object keys, so sorted first)
/// each hold their count until the next explicit level key; levels before
/// the first breakpoint are 0.
fn dense_progression(raw: &RawOptionalFeatureProgressionShape) -> Vec<u8> {
    match raw {
        RawOptionalFeatureProgressionShape::Dense(values) => values.clone(),
        RawOptionalFeatureProgressionShape::Sparse(map) => {
            let mut breakpoints: Vec<(u8, u8)> =
                map.iter().filter_map(|(level, count)| level.parse::<u8>().ok().map(|l| (l, *count))).collect();
            breakpoints.sort_by_key(|(level, _)| *level);

            let mut dense = vec![0u8; 20];
            let mut current = 0u8;
            let mut breakpoints = breakpoints.into_iter().peekable();
            for (level, slot) in dense.iter_mut().enumerate() {
                let level = level as u8 + 1;
                while breakpoints.peek().is_some_and(|(bp, _)| *bp <= level) {
                    current = breakpoints.next().unwrap().1;
                }
                *slot = current;
            }
            dense
        }
    }
}

fn class_from_raw(raw: RawClass) -> Class {
    Class {
        id: slugify(&raw.name),
        canonical_id: format!("{}|{}", raw.name, raw.source),
        hit_die: raw.hd.faces,
        saving_throws: raw.proficiency,
        subclass_title: raw.subclass_title.unwrap_or_else(|| "Subclass".to_string()),
        caster_progression: normalize_caster_progression(raw.caster_progression),
        spellcasting_ability: raw.spellcasting_ability,
        spells_known_progression: raw.spells_known_progression,
        cantrips_known_progression: raw.cantrips_known_progression,
        proficiencies: proficiencies_from_raw(&raw.starting_proficiencies),
        multiclass_proficiencies: raw
            .multiclassing
            .as_ref()
            .and_then(|m| m.proficiencies_gained.as_ref())
            .map(proficiencies_from_raw)
            .unwrap_or_default(),
        starting_equipment: equipment_from_raw(raw.starting_equipment.as_ref()),
        table_groups: raw.class_table_groups.iter().map(table_group_from_raw).collect(),
        optional_feature_progressions: optional_feature_progressions_from_raw(&raw.optional_feature_progression),
        multiclass_ability_prerequisites: raw
            .multiclassing
            .as_ref()
            .map(multiclass_ability_prerequisites_from_raw)
            .unwrap_or_default(),
        name: raw.name,
        source: raw.source,
    }
}

/// Resolves `multiclassing.requirements` into OR-groups of AND-requirements.
/// Flat keys (no `or`) are ANDed; a lone `or` element's keys are OR'd
/// instead; multiple `or` elements are alternative AND-groups.
fn multiclass_ability_prerequisites_from_raw(raw: &RawMulticlassing) -> Vec<Vec<(String, u8)>> {
    let Some(requirements) = raw.requirements.as_ref() else {
        return Vec::new();
    };
    match &requirements.or {
        Some(groups) if groups.len() == 1 => groups[0]
            .iter()
            .map(|(ability, score)| vec![(ability.clone(), *score)])
            .collect(),
        Some(groups) => groups
            .iter()
            .map(|group| group.iter().map(|(ability, score)| (ability.clone(), *score)).collect())
            .collect(),
        None => {
            if requirements.flat.is_empty() {
                Vec::new()
            } else {
                vec![requirements.flat.iter().map(|(ability, score)| (ability.clone(), *score)).collect()]
            }
        }
    }
}

/// 5etools uses fraction-shaped progression codes ("1/2", "1/3"); translate
/// to readable words so the rest of the app (spell slot tables, prepared-
/// caster formulas) can match on them directly instead of parsing fractions.
fn normalize_caster_progression(raw: Option<String>) -> Option<String> {
    raw.map(|value| match value.as_str() {
        "1/2" => "half".to_string(),
        "1/3" => "third".to_string(),
        other => other.to_string(),
    })
}

fn proficiencies_from_raw(raw: &RawStartingProficiencies) -> Proficiencies {
    Proficiencies {
        armor: raw.armor.iter().filter_map(|v| v.as_str()).map(armor_label).collect(),
        weapons: raw.weapons.iter().filter_map(|v| v.as_str()).map(weapon_label).collect(),
        tools: raw
            .tools
            .iter()
            .filter_map(|v| v.as_str())
            .map(|t| capitalize(&clean_tags(t)))
            .collect(),
        skills: skill_grants_from_class(&raw.skills),
    }
}

fn armor_label(code: &str) -> String {
    match code {
        "light" => "Light armor".to_string(),
        "medium" => "Medium armor".to_string(),
        "heavy" => "Heavy armor".to_string(),
        "shield" => "Shields".to_string(),
        other => capitalize(&clean_tags(other)),
    }
}

fn weapon_label(code: &str) -> String {
    match code {
        "simple" => "Simple weapons".to_string(),
        "martial" => "Martial weapons".to_string(),
        other => capitalize(&clean_tags(other)),
    }
}

/// Parses the skill-proficiency choices ("choose N from [...]", "any N", or
/// a fixed list) into structured `SkillGrant`s, preserving the count/options
/// data instead of flattening it to a display string.
fn skill_grants_from_class(skills: &[Value]) -> Vec<SkillGrant> {
    let mut fixed = Vec::new();
    let mut grants = Vec::new();
    for entry in skills {
        match entry {
            Value::String(name) => fixed.push(name.clone()),
            Value::Object(obj) => {
                if let Some(choose) = obj.get("choose") {
                    let count = choose.get("count").and_then(Value::as_u64).unwrap_or(1) as u8;
                    let from: Vec<String> = choose
                        .get("from")
                        .and_then(Value::as_array)
                        .map(|list| list.iter().filter_map(Value::as_str).map(str::to_string).collect())
                        .unwrap_or_default();
                    grants.push(SkillGrant::Choose { count, from });
                } else if let Some(n) = obj.get("any").and_then(Value::as_u64) {
                    grants.push(SkillGrant::Any { count: n as u8 });
                }
            }
            _ => {}
        }
    }
    if !fixed.is_empty() {
        grants.insert(0, SkillGrant::Fixed { skills: fixed });
    }
    grants
}

fn equipment_from_raw(raw: Option<&RawStartingEquipment>) -> Vec<String> {
    let Some(raw) = raw else { return Vec::new() };
    let mut lines: Vec<String> = raw.default.iter().map(|line| clean_tags(line)).collect();
    if let Some(gold) = &raw.gold_alternative {
        lines.push(format!(
            "Alternatively, start with {} gp and buy your own equipment.",
            clean_tags(gold)
        ));
    }
    lines
}

fn table_group_from_raw(raw: &RawTableGroup) -> ClassTableGroup {
    let rows = if raw.rows.is_empty() { &raw.rows_spell_progression } else { &raw.rows };
    ClassTableGroup {
        title: raw.title.clone(),
        col_labels: raw.col_labels.iter().map(|label| clean_tags(label)).collect(),
        rows: rows
            .iter()
            .map(|row| row.iter().map(progression_cell_text).collect())
            .collect(),
    }
}

fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
#[path = "transform_class_tests.rs"]
mod tests;
