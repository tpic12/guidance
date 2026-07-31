use crate::models::entry::Entry;
use serde::{Deserialize, Serialize};

/// A piece of equipment: a mundane item (`baseitem`), a magic item (`item`),
/// or a variant family (`itemGroup`) — all three 5etools content types are
/// unified into one model/table so the compendium is useful for real
/// starting equipment, not just magic items (see TDD.md's Phase 5 notes and
/// the character-equipment ticket this feeds, #11). Combat-relevant fields
/// (`damage`, `base_ac`, `properties`, `magic_bonuses`, `weight_lb`,
/// `value_cp`) are kept typed rather than pre-flattened to display strings,
/// so a future equipment/AC calculator can compute against them without
/// re-importing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Item {
    pub id: String,
    pub canonical_id: String,
    pub name: String,
    /// Short initials of the source book this item was imported from (e.g. "PHB").
    pub source: String,

    /// True for 5etools "itemGroup" records (a variant family, e.g.
    /// "Absorbing Tattoo") rather than one concrete item.
    pub is_group: bool,
    /// Concrete member items, resolved from the raw `items`/`itemsHidden`
    /// arrays, if `is_group`. Empty otherwise.
    #[serde(default)]
    pub group_members: Vec<ItemRef>,
    /// Which item-group(s) this concrete item belongs to (raw `group` field).
    #[serde(default)]
    pub member_of_groups: Vec<String>,

    /// Raw type abbreviation ("M", "SCF", "$A", ...), `|SOURCE` suffix
    /// stripped. `None` for the many wondrous items that carry no type code.
    pub item_type_code: Option<String>,
    /// Resolved display label via the `itemType` lookup, or a synthesized
    /// "Wondrous Item" when `wondrous` is set and no code exists.
    pub item_type_label: Option<String>,

    /// Normalized as 5etools writes it ("none", "common", ..., "artifact",
    /// plus looser source-specific values like "unknown (magic)"/"varies").
    /// Kept as a string, not a closed enum — real source data isn't a clean
    /// closed set. Missing is normalized to `"none"`.
    pub rarity: String,

    pub weight_lb: Option<f64>,
    /// Cost in copper pieces; fractional because some per-unit prices are
    /// genuinely under 1 cp (e.g. a single Ball Bearing).
    pub value_cp: Option<f64>,

    pub requires_attunement: bool,
    /// Descriptive restriction text ("by a druid or ranger"), when
    /// `reqAttune` was a string rather than bare `true`.
    pub attunement_note: Option<String>,

    pub wondrous: bool,
    pub sentient: bool,
    pub curse: bool,

    pub weapon_category: Option<WeaponCategory>,
    /// Property codes + resolved labels — a weapon can carry more than one
    /// (Greatsword: Heavy + Two-Handed).
    #[serde(default)]
    pub properties: Vec<ItemProperty>,
    pub damage: Option<ItemDamage>,
    /// AC this item itself grants when worn (armor/shields) — not a magic
    /// *bonus* to AC (see `magic_bonuses.ac`).
    pub base_ac: Option<i32>,
    pub range: Option<WeaponRange>,

    /// Raw "Name|SOURCE" reference into `baseitem` — kept for display only.
    /// NOT used to fill missing stats: items with a `baseItem` already carry
    /// a full copy of their own combat stats (confirmed via sample
    /// inspection, e.g. "+1 Moon Sickle").
    pub base_item_ref: Option<String>,
    /// Display name of `base_item_ref`, resolved by (name, source) lookup
    /// against the imported `baseitem` list at transform time.
    pub base_item_name: Option<String>,

    #[serde(default)]
    pub magic_bonuses: ItemMagicBonuses,
    pub charges: Option<ItemCharges>,

    /// Classes that can use this as a spellcasting focus (`focus`/`staff`);
    /// display only, not a filter dimension in this pass.
    #[serde(default)]
    pub spellcasting_focus_for: Vec<String>,

    pub srd: bool,

    /// Whatever vehicle-only raw fields (`vehAc`, `vehHp`, `crew`,
    /// `capCargo`, ...) were present, kept verbatim/unstructured — vehicles
    /// are a tiny fraction of this content type and not modeled further in
    /// this pass.
    pub vehicle_extra: Option<serde_json::Value>,

    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ItemRef {
    pub name: String,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ItemProperty {
    pub code: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ItemDamage {
    pub dice: String,
    pub dice_versatile: Option<String>,
    pub damage_type: Option<DamageType>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WeaponRange {
    pub normal: u32,
    pub long: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ItemMagicBonuses {
    pub weapon_attack_and_damage: Option<i32>,
    pub weapon_attack: Option<i32>,
    pub weapon_damage: Option<i32>,
    pub ac: Option<i32>,
    pub spell_attack: Option<i32>,
    pub spell_save_dc: Option<i32>,
    pub saving_throw: Option<i32>,
    pub ability_check: Option<i32>,
    pub proficiency_bonus: Option<i32>,
}

impl ItemMagicBonuses {
    pub fn is_empty(&self) -> bool {
        self.weapon_attack_and_damage.is_none()
            && self.weapon_attack.is_none()
            && self.weapon_damage.is_none()
            && self.ac.is_none()
            && self.spell_attack.is_none()
            && self.spell_save_dc.is_none()
            && self.saving_throw.is_none()
            && self.ability_check.is_none()
            && self.proficiency_bonus.is_none()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ItemCharges {
    /// Display text — usually a plain number ("3"), but sometimes a dice
    /// formula (e.g. a wand recharging to a variable "1d6 + 1" charges).
    pub max: String,
    /// Display text for when charges recharge ("dawn", "1d6 + 4 at dawn", ...).
    pub recharge_trigger: Option<String>,
    pub recharge_amount: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeaponCategory {
    Simple,
    Martial,
}

impl WeaponCategory {
    pub const ALL: [WeaponCategory; 2] = [WeaponCategory::Simple, WeaponCategory::Martial];

    pub fn from_code(code: &str) -> Option<Self> {
        Some(match code {
            "simple" => Self::Simple,
            "martial" => Self::Martial,
            _ => return None,
        })
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Simple => "simple",
            Self::Martial => "martial",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Simple => "Simple",
            Self::Martial => "Martial",
        }
    }
}

/// Full closed 5e damage-type vocabulary (unlike item properties, this is a
/// fixed rules concept, not open source-extensible data — mirrors `School`
/// in `src/models/spell.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DamageType {
    Acid,
    Bludgeoning,
    Cold,
    Fire,
    Force,
    Lightning,
    Necrotic,
    Piercing,
    Poison,
    Psychic,
    Radiant,
    Slashing,
    Thunder,
}

impl DamageType {
    pub const ALL: [DamageType; 13] = [
        DamageType::Acid,
        DamageType::Bludgeoning,
        DamageType::Cold,
        DamageType::Fire,
        DamageType::Force,
        DamageType::Lightning,
        DamageType::Necrotic,
        DamageType::Piercing,
        DamageType::Poison,
        DamageType::Psychic,
        DamageType::Radiant,
        DamageType::Slashing,
        DamageType::Thunder,
    ];

    pub fn from_code(code: &str) -> Option<Self> {
        Some(match code {
            "A" => Self::Acid,
            "B" => Self::Bludgeoning,
            "C" => Self::Cold,
            "F" => Self::Fire,
            "O" => Self::Force,
            "L" => Self::Lightning,
            "N" => Self::Necrotic,
            "P" => Self::Piercing,
            "I" => Self::Poison,
            "Y" => Self::Psychic,
            "R" => Self::Radiant,
            "S" => Self::Slashing,
            "T" => Self::Thunder,
            _ => return None,
        })
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Acid => "acid",
            Self::Bludgeoning => "bludgeoning",
            Self::Cold => "cold",
            Self::Fire => "fire",
            Self::Force => "force",
            Self::Lightning => "lightning",
            Self::Necrotic => "necrotic",
            Self::Piercing => "piercing",
            Self::Poison => "poison",
            Self::Psychic => "psychic",
            Self::Radiant => "radiant",
            Self::Slashing => "slashing",
            Self::Thunder => "thunder",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Acid => "Acid",
            Self::Bludgeoning => "Bludgeoning",
            Self::Cold => "Cold",
            Self::Fire => "Fire",
            Self::Force => "Force",
            Self::Lightning => "Lightning",
            Self::Necrotic => "Necrotic",
            Self::Piercing => "Piercing",
            Self::Poison => "Poison",
            Self::Psychic => "Psychic",
            Self::Radiant => "Radiant",
            Self::Slashing => "Slashing",
            Self::Thunder => "Thunder",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    Item,
    Group,
}

impl ItemKind {
    pub const ALL: [ItemKind; 2] = [ItemKind::Item, ItemKind::Group];

    pub fn label(&self) -> &'static str {
        match self {
            Self::Item => "Item",
            Self::Group => "Item Group",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttunementRequirement {
    Required,
    NotRequired,
}

impl AttunementRequirement {
    pub const ALL: [AttunementRequirement; 2] = [Self::Required, Self::NotRequired];

    pub fn label(&self) -> &'static str {
        match self {
            Self::Required => "Requires Attunement",
            Self::NotRequired => "No Attunement",
        }
    }

    pub fn as_bool(&self) -> bool {
        matches!(self, Self::Required)
    }
}

/// Returned by `list_item_properties` for the Property filter's chip list —
/// filtering needs the stable `code`, the UI wants the readable `label`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemPropertyOption {
    pub code: String,
    pub label: String,
}

/// Search/filter/sort parameters for listing items from the compendium.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ItemQuery {
    #[serde(default)]
    pub search: String,
    #[serde(default)]
    pub sources: Vec<String>,
    #[serde(default)]
    pub kinds: Vec<ItemKind>,
    #[serde(default)]
    pub types: Vec<String>,
    #[serde(default)]
    pub rarities: Vec<String>,
    #[serde(default)]
    pub property_codes: Vec<String>,
    #[serde(default)]
    pub weapon_categories: Vec<WeaponCategory>,
    #[serde(default)]
    pub damage_types: Vec<DamageType>,
    #[serde(default)]
    pub attunement: Vec<AttunementRequirement>,
    #[serde(default)]
    pub sort: ItemSort,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ItemSort {
    #[default]
    NameAsc,
    NameDesc,
    SourceAsc,
    SourceDesc,
    RarityAsc,
    RarityDesc,
    ValueAsc,
    ValueDesc,
    WeightAsc,
    WeightDesc,
}

/// Severity ordering for `rarity` — NOT alphabetical ("artifact" must sort
/// after "legendary", not before "common"). Anything unrecognized ("varies",
/// "unknown (magic)", ...) sorts last. Not stored on `Item` itself — the
/// `items.rarity_rank` column is populated from this at seed time purely so
/// `ORDER BY` can use it.
pub fn rarity_rank(rarity: &str) -> i64 {
    match rarity {
        "none" => 0,
        "common" => 1,
        "uncommon" => 2,
        "rare" => 3,
        "very rare" => 4,
        "legendary" => 5,
        "artifact" => 6,
        _ => 7,
    }
}
