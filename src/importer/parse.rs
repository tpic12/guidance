use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RawSpellFile {
    pub spell: Vec<RawSpell>,
}

#[derive(Debug, Deserialize)]
pub struct RawSpell {
    pub name: String,
    #[serde(default)]
    pub source: String,
    pub level: u8,
    pub school: String,
    #[serde(default)]
    pub time: Vec<RawTime>,
    pub range: Option<RawRange>,
    #[serde(default)]
    pub components: RawComponents,
    #[serde(default)]
    pub duration: Vec<RawDuration>,
    #[serde(default)]
    pub meta: RawMeta,
    #[serde(default)]
    pub entries: Vec<serde_json::Value>,
    #[serde(default, rename = "entriesHigherLevel")]
    pub entries_higher_level: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct RawTime {
    pub number: u8,
    pub unit: String,
    /// Trigger text for reaction spells ("which you take when ...").
    pub condition: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RawRange {
    #[serde(rename = "type")]
    pub kind: String,
    pub distance: Option<RawDistance>,
}

#[derive(Debug, Deserialize)]
pub struct RawDistance {
    #[serde(rename = "type")]
    pub kind: String,
    pub amount: Option<u32>,
}

#[derive(Debug, Default, Deserialize)]
pub struct RawComponents {
    #[serde(default)]
    pub v: bool,
    #[serde(default)]
    pub s: bool,
    pub m: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct RawDuration {
    #[serde(rename = "type")]
    pub kind: String,
    pub duration: Option<RawDurationAmount>,
    #[serde(default)]
    pub concentration: bool,
}

#[derive(Debug, Deserialize)]
pub struct RawDurationAmount {
    #[serde(rename = "type")]
    pub kind: String,
    pub amount: Option<u32>,
}

#[derive(Debug, Default, Deserialize)]
pub struct RawMeta {
    #[serde(default)]
    pub ritual: bool,
}

pub fn parse_spell_array(raw: &str) -> anyhow::Result<Vec<RawSpell>> {
    let file: RawSpellFile = serde_json::from_str(raw)?;
    Ok(file.spell)
}
