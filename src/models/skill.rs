use serde::{Deserialize, Serialize};

/// The 18 D&D 5e skills paired with their governing ability code (one of
/// `character::ABILITY_CODES`). Keys are the lowercase names used verbatim in
/// imported JSON ("sleight of hand", "animal handling", ...), so importer and
/// wizard code can use raw strings from source data as lookup keys directly.
pub const SKILLS: [(&str, &str); 18] = [
    ("acrobatics", "dex"),
    ("animal handling", "wis"),
    ("arcana", "int"),
    ("athletics", "str"),
    ("deception", "cha"),
    ("history", "int"),
    ("insight", "wis"),
    ("intimidation", "cha"),
    ("investigation", "int"),
    ("medicine", "wis"),
    ("nature", "int"),
    ("perception", "wis"),
    ("performance", "cha"),
    ("persuasion", "cha"),
    ("religion", "int"),
    ("sleight of hand", "dex"),
    ("stealth", "dex"),
    ("survival", "wis"),
];

pub fn skill_ability(skill: &str) -> Option<&'static str> {
    SKILLS.iter().find(|(name, _)| *name == skill).map(|(_, ability)| *ability)
}

pub fn is_valid_skill(skill: &str) -> bool {
    SKILLS.iter().any(|(name, _)| *name == skill)
}

/// "sleight of hand" -> "Sleight of Hand". Deliberately separate from the
/// importer's `title_case` (which labels arbitrary free-text tool/language
/// names) — this one only ever needs to handle the 18 known skill names.
pub fn skill_label(skill: &str) -> String {
    skill
        .split(' ')
        .enumerate()
        .map(|(i, word)| match word {
            "of" if i > 0 => word.to_string(),
            _ => {
                let mut chars = word.chars();
                match chars.next() {
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                    None => String::new(),
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// One block of skill-proficiency grants, preserving the fixed/choice/any
/// distinction that raw 5etools JSON carries but which a flattened display
/// string can't. A grants list normally has 0-2 elements (e.g. one `Fixed`
/// and one `Choose`), mirroring the raw array shape 1:1. Skill names stored
/// here are always the canonical lowercase keys from `SKILLS`, not display
/// labels — use `skill_label`/`describe_skill_grants` to render them.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind")]
pub enum SkillGrant {
    /// Skills granted outright, no player choice.
    Fixed { skills: Vec<String> },
    /// Choose `count` from a specific list of candidate skills.
    Choose { count: u8, from: Vec<String> },
    /// Choose `count` from all 18 skills ("any": N in source data).
    Any { count: u8 },
}

/// Renders grants back into the "Choose 2 from Athletics, ..." / "Any 3" /
/// "Insight, History" text compendium pages show — derived from the
/// structured grants so display can never drift out of sync with them.
pub fn describe_skill_grants(grants: &[SkillGrant]) -> String {
    grants
        .iter()
        .map(|grant| match grant {
            SkillGrant::Fixed { skills } => {
                skills.iter().map(|s| skill_label(s)).collect::<Vec<_>>().join(", ")
            }
            SkillGrant::Choose { count, from } => {
                let options = from.iter().map(|s| skill_label(s)).collect::<Vec<_>>().join(", ");
                format!("Choose {count} from {options}")
            }
            SkillGrant::Any { count } => format!("Any {count}"),
        })
        .collect::<Vec<_>>()
        .join("; ")
}

#[cfg(test)]
#[path = "skill_tests.rs"]
mod tests;
