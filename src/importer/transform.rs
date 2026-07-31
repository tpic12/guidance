use crate::importer::entries::{clean_tags, entries_from_values};
use crate::importer::parse::{RawComponents, RawDuration, RawRange, RawSpell};
use crate::models::spell::{CastingType, Components, Duration, Range, School, Spell};
use anyhow::{bail, Context};

pub fn spells_from_parsed(raw_spells: Vec<RawSpell>) -> anyhow::Result<Vec<Spell>> {
    raw_spells.into_iter().map(spell_from_raw).collect()
}

fn spell_from_raw(raw: RawSpell) -> anyhow::Result<Spell> {
    let school = school_from_code(&raw.school)
        .with_context(|| format!("unknown school code '{}' for spell '{}'", raw.school, raw.name))?;

    let (casting_time, casting_type, casting_condition) = match raw.time.first() {
        Some(time) => (
            time.number,
            casting_type_from_unit(&time.unit),
            time.condition.as_deref().map(clean_tags),
        ),
        None => (1, CastingType::Action, None),
    };

    Ok(Spell {
        id: slugify(&raw.name),
        canonical_id: format!("{}|{}", raw.name, raw.source),
        name: raw.name,
        source: raw.source,
        level: raw.level,
        school,
        casting_time,
        casting_type,
        casting_condition,
        range: range_from_raw(raw.range),
        components: components_from_raw(raw.components),
        duration: duration_from_raw(raw.duration.into_iter().next()),
        ritual: raw.meta.ritual,
        description: entries_from_values(&raw.entries),
        higher_level: entries_from_values(&raw.entries_higher_level),
    })
}

fn school_from_code(code: &str) -> anyhow::Result<School> {
    Ok(match code {
        "A" => School::Abjuration,
        "C" => School::Conjuration,
        "D" => School::Divination,
        "E" => School::Enchantment,
        "V" => School::Evocation,
        "I" => School::Illusion,
        "N" => School::Necromancy,
        "T" => School::Transmutation,
        other => bail!("unrecognized school code: {other}"),
    })
}

fn casting_type_from_unit(unit: &str) -> CastingType {
    match unit {
        "bonus" => CastingType::BonusAction,
        "reaction" => CastingType::Reaction,
        "minute" => CastingType::Minute,
        "hour" => CastingType::Hour,
        "day" => CastingType::Day,
        "week" => CastingType::Week,
        "month" => CastingType::Month,
        // "action", plus anything unrecognized ("round", "special", ...).
        _ => CastingType::Action,
    }
}

fn range_from_raw(raw: Option<RawRange>) -> Range {
    match raw {
        Some(raw) => Range {
            distance: raw.distance.as_ref().and_then(|d| d.amount),
            unit: raw.distance.map(|d| d.kind),
            kind: raw.kind,
        },
        None => Range { kind: "self".to_string(), distance: None, unit: None },
    }
}

fn components_from_raw(raw: RawComponents) -> Components {
    Components {
        verbal: raw.v,
        somatic: raw.s,
        material: raw.m.map(|m| match m.as_str() {
            Some(text) => text.to_string(),
            None => m.get("text").and_then(|t| t.as_str()).unwrap_or_default().to_string(),
        }),
    }
}

fn duration_from_raw(raw: Option<RawDuration>) -> Duration {
    match raw {
        Some(raw) => Duration {
            amount: raw.duration.as_ref().and_then(|d| d.amount),
            unit: raw.duration.map(|d| d.kind),
            concentration: raw.concentration,
            kind: raw.kind,
        },
        None => Duration { kind: "instant".to_string(), amount: None, unit: None, concentration: false },
    }
}

pub(crate) fn slugify(name: &str) -> String {
    let mut slug = String::with_capacity(name.len());
    let mut last_was_dash = false;
    for ch in name.to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            last_was_dash = false;
        } else if !last_was_dash {
            slug.push('-');
            last_was_dash = true;
        }
    }
    slug.trim_matches('-').to_string()
}

#[cfg(test)]
#[path = "transform_tests.rs"]
mod tests;
