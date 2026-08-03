use crate::importer::entries::entries_from_values;
use crate::importer::parse_species::RawSpecies;
use crate::importer::proficiency_grants::language_grants_from_raw;
use crate::importer::transform::slugify;
use crate::models::class::ability_label;
use crate::models::species::{AbilityBonusGrant, Species};
use serde_json::Value;

pub fn species_from_parsed(raws: Vec<RawSpecies>) -> anyhow::Result<Vec<Species>> {
    Ok(raws
        .into_iter()
        .filter(|raw| raw.copy.is_none())
        .map(species_from_raw)
        .collect())
}

fn species_from_raw(raw: RawSpecies) -> Species {
    Species {
        // Reprinted species share a name across books (Aasimar, Goblin, ...),
        // so the slug needs the source to stay unique.
        id: slugify(&format!("{} {}", raw.name, raw.source)),
        canonical_id: format!("{}|{}", raw.name, raw.source),
        ability: ability_score_label(&raw.ability),
        ability_bonuses: ability_bonus_grants(&raw.ability),
        size: size_label(&raw.size),
        speed: speed_label(&raw.speed),
        darkvision: raw.darkvision,
        languages: language_grants_from_raw(&raw.language_proficiencies),
        entries: entries_from_values(&raw.entries),
        name: raw.name,
        source: raw.source,
    }
}

/// Structured form of the same raw data `ability_score_label` renders to
/// text — for the character builder to actually apply, not just display.
fn ability_bonus_grants(abilities: &[Value]) -> Vec<AbilityBonusGrant> {
    let mut grants = Vec::new();
    for grant in abilities.iter().filter_map(Value::as_object) {
        for (key, value) in grant {
            if key == "choose" {
                continue;
            }
            if let Some(amount) = value.as_i64() {
                grants.push(AbilityBonusGrant::Fixed { code: key.clone(), amount: amount as i8 });
            }
        }
        if let Some(choose) = grant.get("choose") {
            let count = choose.get("count").and_then(Value::as_u64).unwrap_or(1) as u8;
            let amount = choose.get("amount").and_then(Value::as_i64).unwrap_or(1) as i8;
            let from: Vec<String> = choose
                .get("from")
                .and_then(Value::as_array)
                .map(|from| from.iter().filter_map(Value::as_str).map(String::from).collect())
                .unwrap_or_default();
            grants.push(AbilityBonusGrant::Choose { count, amount, from });
        }
    }
    grants
}

/// Compact summary for the species table, e.g. "Dexterity +2, Wisdom +1" or
/// "Charisma +2, Choose Strength or Dexterity +1". Fixed increases are listed
/// before any `choose` grant.
fn ability_score_label(abilities: &[Value]) -> Option<String> {
    let mut parts = Vec::new();
    for grant in abilities.iter().filter_map(Value::as_object) {
        for (key, value) in grant {
            if key == "choose" {
                continue;
            }
            if let Some(amount) = value.as_i64() {
                parts.push(format!("{} {amount:+}", ability_label(key)));
            }
        }
        if let Some(choose) = grant.get("choose") {
            let count = choose.get("count").and_then(Value::as_u64).unwrap_or(1);
            let amount = choose.get("amount").and_then(Value::as_i64).unwrap_or(1);
            let names: Vec<&str> = choose
                .get("from")
                .and_then(Value::as_array)
                .map(|from| {
                    from.iter()
                        .filter_map(Value::as_str)
                        .map(ability_label)
                        .collect()
                })
                .unwrap_or_default();
            // All six abilities listed means a free choice.
            parts.push(match (count, names.len()) {
                (1, n) if n == 0 || n >= 6 => format!("Choose any {amount:+}"),
                (_, n) if n == 0 || n >= 6 => format!("Choose any {count} {amount:+}"),
                (1, _) => format!("Choose {} {amount:+}", names.join(" or ")),
                _ => format!("Choose {count} from {} {amount:+}", names.join(", ")),
            });
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(", "))
    }
}

fn size_label(codes: &[String]) -> Option<String> {
    if codes.is_empty() {
        return None;
    }
    let names: Vec<&str> = codes
        .iter()
        .map(|code| match code.as_str() {
            "T" => "Tiny",
            "S" => "Small",
            "M" => "Medium",
            "L" => "Large",
            "H" => "Huge",
            "G" => "Gargantuan",
            "V" => "Varies",
            other => other,
        })
        .collect();
    Some(names.join(" or "))
}

/// Speed is either a bare walking speed or a map of movement modes, where
/// `true` means "equal to walking speed".
fn speed_label(speed: &Value) -> Option<String> {
    if let Some(walk) = speed.as_u64() {
        return Some(format!("{walk} ft."));
    }
    let speeds = speed.as_object()?;
    let walk = speeds.get("walk").and_then(Value::as_u64);
    let mut parts = Vec::new();
    if let Some(walk) = walk {
        parts.push(format!("{walk} ft."));
    }
    for mode in ["burrow", "climb", "fly", "swim"] {
        let Some(value) = speeds.get(mode) else {
            continue;
        };
        let distance = match value {
            Value::Bool(true) => walk,
            other => other.as_u64(),
        };
        if let Some(distance) = distance {
            parts.push(format!("{mode} {distance} ft."));
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(", "))
    }
}

#[cfg(test)]
#[path = "transform_species_tests.rs"]
mod tests;
