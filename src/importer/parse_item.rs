use anyhow::Context;
use serde::Deserialize;
use serde_json::{Map, Value};

/// Returns the raw (unresolved) `item` array and `itemGroup` array as
/// untyped [`Value`]s — `item` still needs `_copy` resolution
/// (`transform_item::resolve_item_copies`) before it can be deserialized
/// into [`RawItem`].
pub fn parse_item_file(raw: &str) -> anyhow::Result<(Vec<Value>, Vec<Value>)> {
    let root: Value = serde_json::from_str(raw)?;
    let items = root
        .get("item")
        .and_then(Value::as_array)
        .context("missing top-level item array")?
        .clone();
    let groups = root
        .get("itemGroup")
        .and_then(Value::as_array)
        .context("missing top-level itemGroup array")?
        .clone();
    Ok((items, groups))
}

#[derive(Debug, Deserialize)]
pub struct RawItemProperty {
    pub abbreviation: String,
    #[serde(default)]
    pub entries: Vec<Value>,
}

#[derive(Debug, Deserialize)]
pub struct RawItemType {
    pub abbreviation: String,
    pub name: String,
}

/// Returns the raw `baseitem` array (no `_copy` present in this table, so no
/// resolution pass is needed before deserializing into [`RawItem`]) plus the
/// `itemProperty`/`itemType` lookup rows used to resolve abbreviation codes
/// into display labels at transform time. `itemTypeAdditionalEntries`/
/// `itemEntry` are intentionally not parsed here — see the known-gaps note
/// in `transform_item.rs`.
pub fn parse_base_item_file(
    raw: &str,
) -> anyhow::Result<(Vec<Value>, Vec<RawItemProperty>, Vec<RawItemType>)> {
    let root: Value = serde_json::from_str(raw)?;
    let base_items = root
        .get("baseitem")
        .and_then(Value::as_array)
        .context("missing top-level baseitem array")?
        .clone();
    let properties: Vec<RawItemProperty> = root
        .get("itemProperty")
        .and_then(Value::as_array)
        .context("missing itemProperty array")?
        .iter()
        .map(|value| {
            serde_json::from_value(value.clone()).context("failed to parse itemProperty")
        })
        .collect::<anyhow::Result<_>>()?;
    let types: Vec<RawItemType> = root
        .get("itemType")
        .and_then(Value::as_array)
        .context("missing itemType array")?
        .iter()
        .map(|value| serde_json::from_value(value.clone()).context("failed to parse itemType"))
        .collect::<anyhow::Result<_>>()?;
    Ok((base_items, properties, types))
}

/// Deserialized *after* `_copy` merging (for the `item` array) — shared by
/// `item`, `itemGroup`, and `baseitem` raw records, since their field sets
/// overlap enough and unrecognized fields are ignored like every other
/// `Raw*` struct in this codebase.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RawItem {
    pub name: String,
    #[serde(default)]
    pub source: String,
    #[serde(default, rename = "type")]
    pub item_type: Option<String>,
    pub rarity: Option<String>,
    pub weight: Option<f64>,
    pub value: Option<f64>,
    pub ac: Option<i32>,
    pub dmg1: Option<String>,
    pub dmg2: Option<String>,
    #[serde(default, rename = "dmgType")]
    pub dmg_type: Option<String>,
    #[serde(default)]
    pub property: Vec<String>,
    pub range: Option<String>,
    #[serde(default, rename = "reqAttune")]
    pub req_attune: Option<Value>,
    #[serde(default, rename = "weaponCategory")]
    pub weapon_category: Option<String>,
    #[serde(default, rename = "baseItem")]
    pub base_item: Option<String>,
    #[serde(default, rename = "bonusWeapon")]
    pub bonus_weapon: Option<String>,
    #[serde(default, rename = "bonusWeaponAttack")]
    pub bonus_weapon_attack: Option<String>,
    #[serde(default, rename = "bonusWeaponDamage")]
    pub bonus_weapon_damage: Option<String>,
    #[serde(default, rename = "bonusAc")]
    pub bonus_ac: Option<String>,
    #[serde(default, rename = "bonusSpellAttack")]
    pub bonus_spell_attack: Option<String>,
    #[serde(default, rename = "bonusSpellSaveDc")]
    pub bonus_spell_save_dc: Option<String>,
    #[serde(default, rename = "bonusSavingThrow")]
    pub bonus_saving_throw: Option<String>,
    #[serde(default, rename = "bonusAbilityCheck")]
    pub bonus_ability_check: Option<String>,
    #[serde(default, rename = "bonusProficiencyBonus")]
    pub bonus_proficiency_bonus: Option<String>,
    /// Usually a plain integer, but sometimes a dice-formula string (e.g. a
    /// wand recharging to a variable number of charges) — kept untyped and
    /// rendered via `value_display_text` in `transform_item.rs`.
    pub charges: Option<Value>,
    pub recharge: Option<String>,
    /// Usually a display string ("1d6 + 4"), but sometimes a bare integer
    /// (e.g. Azuredge's `rechargeAmount: 3`) — kept untyped and rendered via
    /// `value_display_text` in `transform_item.rs`.
    #[serde(default, rename = "rechargeAmount")]
    pub recharge_amount: Option<Value>,
    #[serde(default)]
    pub curse: bool,
    #[serde(default)]
    pub sentient: bool,
    #[serde(default)]
    pub wondrous: bool,
    pub srd: Option<Value>,
    pub staff: Option<Value>,
    pub focus: Option<Value>,
    #[serde(default)]
    pub group: Vec<String>,
    #[serde(default)]
    pub entries: Vec<Value>,
    // itemGroup-only:
    #[serde(default)]
    pub items: Vec<String>,
    /// Usually an array of "Name|SOURCE" member refs, but sometimes a bare
    /// `true` (e.g. "Lantern of Tracking") meaning "members intentionally
    /// not enumerated" rather than naming any — kept untyped and only the
    /// array shape is used (see `item_from_raw`).
    #[serde(default, rename = "itemsHidden")]
    pub items_hidden: Option<Value>,
    /// Catches everything not named above (vehAc, vehHp, crew, capCargo,
    /// capPassenger, reqAttuneTags, attachedSpells, referenceSources, page,
    /// ...) — `transform_item.rs` picks the vehicle-shaped subset out of this
    /// for `vehicle_extra`; the rest is a documented scope cut (see the
    /// known-gaps note there).
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
