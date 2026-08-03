use crate::importer::parse_language::RawLanguage;
use crate::importer::transform::slugify;
use crate::models::language::{Language, LanguageType};

pub fn languages_from_parsed(raws: Vec<RawLanguage>) -> anyhow::Result<Vec<Language>> {
    Ok(raws.into_iter().map(language_from_raw).collect())
}

fn language_from_raw(raw: RawLanguage) -> Language {
    Language {
        // Languages get reprinted across source books far more than most
        // compendium content (Common, Draconic, ... in a dozen+ books), so
        // (unlike Feat's plain `slugify(name)`) the slug needs the source to
        // stay unique — same reasoning as `transform_species`'s species id.
        id: slugify(&format!("{} {}", raw.name, raw.source)),
        canonical_id: format!("{}|{}", raw.name, raw.source),
        language_type: raw.language_type.as_deref().and_then(language_type_from_str),
        script: raw.script,
        name: raw.name,
        source: raw.source,
    }
}

fn language_type_from_str(value: &str) -> Option<LanguageType> {
    match value {
        "standard" => Some(LanguageType::Standard),
        "exotic" => Some(LanguageType::Exotic),
        "rare" => Some(LanguageType::Rare),
        "secret" => Some(LanguageType::Secret),
        _ => None,
    }
}

#[cfg(test)]
#[path = "transform_language_tests.rs"]
mod tests;
