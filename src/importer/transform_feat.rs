use crate::importer::entries::{clean_tags, entries_from_values};
use crate::importer::parse_feat::RawFeat;
use crate::importer::transform::slugify;
use crate::importer::transform_background::title_case;
use crate::models::class::ability_label;
use crate::models::feat::Feat;
use serde_json::Value;

pub fn feats_from_parsed(raws: Vec<RawFeat>) -> anyhow::Result<Vec<Feat>> {
    Ok(raws.into_iter().map(feat_from_raw).collect())
}

fn feat_from_raw(raw: RawFeat) -> Feat {
    Feat {
        id: slugify(&raw.name),
        canonical_id: format!("{}|{}", raw.name, raw.source),
        prerequisite: prerequisite_label(&raw.prerequisite),
        ability: ability_increase_label(&raw.ability),
        entries: entries_from_values(&raw.entries),
        name: raw.name,
        source: raw.source,
    }
}

/// Each element of the prerequisite array is an alternative (OR); the keys
/// within one element are requirements that must all hold (AND).
fn prerequisite_label(prerequisites: &[Value]) -> Option<String> {
    let options: Vec<String> = prerequisites
        .iter()
        .filter_map(|option| {
            let parts = prerequisite_option_parts(option.as_object()?);
            if parts.is_empty() {
                None
            } else {
                Some(parts.join(", "))
            }
        })
        .collect();
    if options.is_empty() {
        None
    } else {
        Some(options.join(" or "))
    }
}

fn prerequisite_option_parts(option: &serde_json::Map<String, Value>) -> Vec<String> {
    let mut parts = Vec::new();

    if let Some(level) = option.get("level").and_then(Value::as_u64) {
        parts.push(format!("{} level", ordinal(level)));
    }
    if let Some(races) = option.get("race").and_then(Value::as_array) {
        let names: Vec<String> = races
            .iter()
            .filter_map(|race| {
                let name = title_case(race.get("name")?.as_str()?);
                Some(match race.get("subrace").and_then(Value::as_str) {
                    Some(subrace) => format!("{name} ({subrace})"),
                    None => name,
                })
            })
            .collect();
        parts.push(names.join(" or "));
    }
    if let Some(abilities) = option.get("ability").and_then(Value::as_array) {
        let scores: Vec<(String, u64)> = abilities
            .iter()
            .filter_map(Value::as_object)
            .flatten()
            .filter_map(|(code, score)| Some((ability_label(code).to_string(), score.as_u64()?)))
            .collect();
        if let Some((_, score)) = scores.first() {
            let names = scores
                .iter()
                .map(|(name, _)| name.as_str())
                .collect::<Vec<_>>();
            parts.push(format!("{} {score} or higher", names.join(" or ")));
        }
    }
    if let Some(proficiencies) = option.get("proficiency").and_then(Value::as_array) {
        for proficiency in proficiencies.iter().filter_map(Value::as_object).flatten() {
            let (kind, grade) = proficiency;
            let grade = grade.as_str().unwrap_or_default();
            parts.push(match kind.as_str() {
                "armor" => format!("Proficiency with {grade} armor"),
                "weapon" | "weaponGroup" => format!("Proficiency with a {grade} weapon"),
                other => format!("Proficiency with {grade} {other}"),
            });
        }
    }
    if option.get("spellcasting").is_some() {
        parts.push("The ability to cast at least one spell".to_string());
    }
    if option.get("spellcasting2020").is_some() {
        parts.push("Spellcasting or Pact Magic feature".to_string());
    }
    if option.get("spellcastingFeature").is_some() {
        parts.push("Spellcasting feature".to_string());
    }
    if let Some(feats) = option.get("feat").and_then(Value::as_array) {
        // Encoded as "name|source|display name": prefer the display form.
        for feat in feats.iter().filter_map(Value::as_str) {
            let mut fields = feat.split('|');
            let name = fields.next().unwrap_or_default();
            parts.push(format!(
                "{} feat",
                title_case(fields.nth(1).unwrap_or(name))
            ));
        }
    }
    if let Some(backgrounds) = option.get("background").and_then(Value::as_array) {
        for background in backgrounds {
            if let Some(name) = background.get("name").and_then(Value::as_str) {
                parts.push(format!("{name} background"));
            }
        }
    }
    if let Some(campaigns) = option.get("campaign").and_then(Value::as_array) {
        for campaign in campaigns.iter().filter_map(Value::as_str) {
            parts.push(format!("{campaign} campaign"));
        }
    }
    if let Some(other) = option.get("other").and_then(Value::as_str) {
        parts.push(clean_tags(other));
    }

    parts
}

/// The ability score increase a feat grants: either fixed ("con": 1) or a
/// `choose` over a list of ability codes.
fn ability_increase_label(abilities: &[Value]) -> Option<String> {
    let mut sentences = Vec::new();
    for grant in abilities.iter().filter_map(Value::as_object) {
        for (key, value) in grant {
            if key == "choose" {
                let amount = value.get("amount").and_then(Value::as_u64).unwrap_or(1);
                let from = value.get("from").and_then(Value::as_array);
                let scope = match from {
                    // All six abilities listed means a free choice.
                    Some(from) if from.len() >= 6 => "one ability score of your choice".to_string(),
                    Some(from) => {
                        let names: Vec<&str> = from
                            .iter()
                            .filter_map(Value::as_str)
                            .map(ability_label)
                            .collect();
                        format!("your {} score", names.join(" or "))
                    }
                    None => "one ability score of your choice".to_string(),
                };
                sentences.push(format!("Increase {scope} by {amount}, to a maximum of 20."));
            } else if let Some(amount) = value.as_u64() {
                sentences.push(format!(
                    "Increase your {} score by {amount}, to a maximum of 20.",
                    ability_label(key)
                ));
            }
        }
    }
    if sentences.is_empty() {
        None
    } else {
        Some(sentences.join(" "))
    }
}

fn ordinal(n: u64) -> String {
    let suffix = match n % 100 {
        11..=13 => "th",
        _ => match n % 10 {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        },
    };
    format!("{n}{suffix}")
}

#[cfg(test)]
#[path = "transform_feat_tests.rs"]
mod tests;
