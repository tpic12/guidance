use crate::importer::entries::{clean_tags, entries_from_values};
use crate::importer::parse_item::{RawItem, RawItemProperty, RawItemType};
use crate::importer::transform::slugify;
use crate::models::item::*;
use anyhow::Context;
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};

/// Merges `item` (mostly magic), `itemGroup` (variant families), and
/// `baseitem` (mundane gear) into one flat `Item` list — see `Item`'s doc
/// comment for why these three 5etools content types are unified. `_copy`
/// inheritance (see `resolve_item_copies`) only ever appears on the `item`
/// array, so only `raw_items` goes through that pre-pass.
pub fn items_from_parsed(
    raw_items: Vec<Value>,
    raw_groups: Vec<Value>,
    raw_base_items: Vec<Value>,
    properties: &[RawItemProperty],
    types: &[RawItemType],
) -> anyhow::Result<Vec<Item>> {
    let property_labels = property_label_map(properties);
    let type_labels = type_label_map(types);
    let base_item_names = base_item_name_index(&raw_base_items);

    let resolved_items = resolve_item_copies(raw_items);

    let mut used_ids: HashSet<String> = HashSet::new();
    let mut out = Vec::new();

    for value in resolved_items {
        let raw = deserialize_raw_item(&value)?;
        out.push(item_from_raw(raw, false, &property_labels, &type_labels, &base_item_names, &mut used_ids));
    }
    for value in raw_groups {
        let raw = deserialize_raw_item(&value)?;
        out.push(item_from_raw(raw, true, &property_labels, &type_labels, &base_item_names, &mut used_ids));
    }
    for value in raw_base_items {
        let raw = deserialize_raw_item(&value)?;
        out.push(item_from_raw(raw, false, &property_labels, &type_labels, &base_item_names, &mut used_ids));
    }

    Ok(out)
}

fn deserialize_raw_item(value: &Value) -> anyhow::Result<RawItem> {
    let name = value.get("name").and_then(Value::as_str).unwrap_or("<unnamed>").to_string();
    serde_json::from_value(value.clone()).with_context(|| format!("failed to parse item '{name}'"))
}

/// Resolves 5etools' `_copy` inheritance recursively (parent-first), with
/// memoization and a cycle guard. Only the `item` array carries `_copy` (not
/// `itemGroup`/`baseitem`), so this is the only place copy-resolution logic
/// lives. `_copy` chains recursively in the real data (a child copying a
/// parent that itself copies a grandparent), which is why this can't be a
/// single-hop lookup.
fn resolve_item_copies(raws: Vec<Value>) -> Vec<Value> {
    let by_key: HashMap<(String, String), Value> =
        raws.iter().map(|v| (name_source_key(v), v.clone())).collect();
    let mut resolved: HashMap<(String, String), Value> = HashMap::new();
    for key in by_key.keys().cloned().collect::<Vec<_>>() {
        let mut visiting = HashSet::new();
        resolve_one(&key, &by_key, &mut resolved, &mut visiting);
    }
    raws.iter()
        .map(|v| resolved.get(&name_source_key(v)).cloned().unwrap_or_else(|| v.clone()))
        .collect()
}

fn resolve_one(
    key: &(String, String),
    by_key: &HashMap<(String, String), Value>,
    resolved: &mut HashMap<(String, String), Value>,
    visiting: &mut HashSet<(String, String)>,
) -> Value {
    if let Some(done) = resolved.get(key) {
        return done.clone();
    }
    let Some(raw) = by_key.get(key).cloned() else { return Value::Null };
    let Some(copy) = raw.get("_copy") else {
        resolved.insert(key.clone(), raw.clone());
        return raw;
    };

    let parent_key = name_source_key(copy);
    let parent = if visiting.contains(&parent_key) || parent_key == *key {
        // Cycle guard: a `_copy` reference pointing at a key already being
        // resolved higher up the call stack (or at itself) is treated as
        // having no parent, rather than recursing forever.
        Value::Null
    } else {
        visiting.insert(key.clone());
        let parent = resolve_one(&parent_key, by_key, resolved, visiting);
        visiting.remove(key);
        parent
    };

    let mut merged: Map<String, Value> = parent.as_object().cloned().unwrap_or_default();
    if let Some(child_obj) = raw.as_object() {
        for (k, v) in child_obj {
            if k != "_copy" {
                merged.insert(k.clone(), v.clone());
            }
        }
    }
    if let Some(mod_spec) = copy.get("_mod") {
        apply_entries_mod(&mut merged, mod_spec);
    }

    let result = Value::Object(merged);
    resolved.insert(key.clone(), result.clone());
    result
}

/// Supports exactly `insertArr` (splice at `index`, or append if the index
/// is negative/out of range) and `appendArr` (append — the same operation
/// with an implicit end index), the two `_mod.entries.mode` values that
/// actually occur in the real dataset. Any other mode (`replaceArr`, one
/// occurrence): a documented known-gap, left as a no-op rather than a crash
/// — the merged entries stand as they are post-parent-merge.
fn apply_entries_mod(merged: &mut Map<String, Value>, mod_spec: &Value) {
    let Some(entries_mod) = mod_spec.get("entries") else { return };
    let mode = entries_mod.get("mode").and_then(Value::as_str).unwrap_or("");
    if mode != "insertArr" && mode != "appendArr" {
        return;
    }

    let entries = merged.entry("entries").or_insert_with(|| Value::Array(Vec::new()));
    let Value::Array(entries_arr) = entries else { return };

    let items = entries_mod.get("items").cloned().unwrap_or(Value::Null);
    let to_insert: Vec<Value> = match items {
        Value::Array(items) => items,
        other => vec![other],
    };

    let index = entries_mod.get("index").and_then(Value::as_i64).unwrap_or(-1);
    let at = if index < 0 || index as usize > entries_arr.len() { entries_arr.len() } else { index as usize };
    for (offset, item) in to_insert.into_iter().enumerate() {
        entries_arr.insert(at + offset, item);
    }
}

fn name_source_key(v: &Value) -> (String, String) {
    let name = v.get("name").and_then(Value::as_str).unwrap_or_default().to_lowercase();
    let source = v.get("source").and_then(Value::as_str).unwrap_or_default().to_lowercase();
    (name, source)
}

/// Flat `abbreviation -> name` map. Unlike `item`, `itemType`'s handful of
/// `_copy` entries (2 of 34) always carry their own `name` directly too —
/// confirmed via inspection — so no copy-resolution is needed for this
/// lookup table.
fn type_label_map(types: &[RawItemType]) -> HashMap<String, String> {
    types.iter().map(|t| (t.abbreviation.clone(), t.name.clone())).collect()
}

/// `itemProperty`'s human label isn't a top-level field — it lives at
/// `entries[0].name` (e.g. abbreviation "H" -> `entries: [{name: "Heavy", ...}]`).
fn property_label_map(properties: &[RawItemProperty]) -> HashMap<String, String> {
    properties
        .iter()
        .map(|property| {
            let label = property
                .entries
                .first()
                .and_then(|entry| entry.get("name"))
                .and_then(Value::as_str)
                .map(clean_tags)
                .unwrap_or_else(|| property.abbreviation.clone());
            (property.abbreviation.clone(), label)
        })
        .collect()
}

/// `(name.lower, source.lower) -> canonical display name`, for resolving a
/// `baseItem: "sickle|PHB"` reference to "Sickle" for display. Not used to
/// backfill missing combat stats — items with a `baseItem` already carry a
/// full copy of their own stats (see `Item::base_item_name`'s doc comment).
fn base_item_name_index(raw_base_items: &[Value]) -> HashMap<(String, String), String> {
    raw_base_items
        .iter()
        .filter_map(|v| {
            let name = v.get("name").and_then(Value::as_str)?.to_string();
            let source = v.get("source").and_then(Value::as_str).unwrap_or_default().to_lowercase();
            Some(((name.to_lowercase(), source), name))
        })
        .collect()
}

/// Splits a 5etools `"Name|SOURCE"` reference string into its parts (source
/// is optional in some contexts, e.g. a bare item name).
fn parse_ref_string(raw: &str) -> ItemRef {
    let mut parts = raw.splitn(2, '|');
    let name = parts.next().unwrap_or_default().to_string();
    let source = parts.next().map(str::to_string);
    ItemRef { name, source }
}

/// Deterministic id assignment with a collision-safe fallback. Unifying
/// `item` + `itemGroup` + `baseitem` into one id space surfaces real name
/// collisions in the actual fixture data (e.g. "Trinket" appears 4x across
/// different sources, but at least one exact name+source collision also
/// exists) — a bare `slugify(name)`/`slugify(name+source)` isn't guaranteed
/// unique, so collisions get an incrementing suffix.
fn unique_id(base_slug: String, used: &mut HashSet<String>) -> String {
    if used.insert(base_slug.clone()) {
        return base_slug;
    }
    let mut n = 2;
    loop {
        let candidate = format!("{base_slug}-{n}");
        if used.insert(candidate.clone()) {
            return candidate;
        }
        n += 1;
    }
}

/// Parses a magic bonus string ("+1", "+2", ...) into a signed integer.
fn parse_bonus(raw: &str) -> Option<i32> {
    raw.trim().trim_start_matches('+').parse::<i32>().ok()
}

/// Renders a loosely-typed display field (usually a string, sometimes a
/// bare number, e.g. Azuredge's `rechargeAmount: 3`) as text.
fn value_display_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(clean_tags(text)),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn item_from_raw(
    raw: RawItem,
    is_group: bool,
    property_labels: &HashMap<String, String>,
    type_labels: &HashMap<String, String>,
    base_item_names: &HashMap<(String, String), String>,
    used_ids: &mut HashSet<String>,
) -> Item {
    let id = unique_id(slugify(&format!("{}-{}", raw.name, raw.source)), used_ids);
    let canonical_id = format!("{}|{}", raw.name, raw.source);

    let item_type_code = raw.item_type.as_deref().map(|t| t.split('|').next().unwrap_or(t).to_string());
    let item_type_label = match &item_type_code {
        Some(code) => Some(type_labels.get(code).cloned().unwrap_or_else(|| code.clone())),
        None if raw.wondrous => Some("Wondrous Item".to_string()),
        None => None,
    };

    let (requires_attunement, attunement_note) = match &raw.req_attune {
        None => (false, None),
        Some(Value::Bool(required)) => (*required, None),
        Some(Value::String(note)) => (true, Some(clean_tags(note))),
        Some(_) => (true, None),
    };

    let properties: Vec<ItemProperty> = raw
        .property
        .iter()
        .map(|code| {
            let code = code.split('|').next().unwrap_or(code).to_string();
            let label = property_labels.get(&code).cloned().unwrap_or_else(|| code.clone());
            ItemProperty { code, label }
        })
        .collect();

    let damage = raw.dmg1.as_ref().map(|dice| ItemDamage {
        dice: dice.clone(),
        dice_versatile: raw.dmg2.clone(),
        damage_type: raw.dmg_type.as_deref().and_then(DamageType::from_code),
    });

    let range = raw.range.as_ref().and_then(|range| {
        let mut parts = range.split('/');
        let normal = parts.next()?.trim().parse::<u32>().ok()?;
        let long = parts.next().and_then(|long| long.trim().parse::<u32>().ok());
        Some(WeaponRange { normal, long })
    });

    let base_item_name = raw.base_item.as_ref().and_then(|reference| {
        let item_ref = parse_ref_string(reference);
        let source = item_ref.source.unwrap_or_default().to_lowercase();
        base_item_names.get(&(item_ref.name.to_lowercase(), source)).cloned()
    });

    let magic_bonuses = ItemMagicBonuses {
        weapon_attack_and_damage: raw.bonus_weapon.as_deref().and_then(parse_bonus),
        weapon_attack: raw.bonus_weapon_attack.as_deref().and_then(parse_bonus),
        weapon_damage: raw.bonus_weapon_damage.as_deref().and_then(parse_bonus),
        ac: raw.bonus_ac.as_deref().and_then(parse_bonus),
        spell_attack: raw.bonus_spell_attack.as_deref().and_then(parse_bonus),
        spell_save_dc: raw.bonus_spell_save_dc.as_deref().and_then(parse_bonus),
        saving_throw: raw.bonus_saving_throw.as_deref().and_then(parse_bonus),
        ability_check: raw.bonus_ability_check.as_deref().and_then(parse_bonus),
        proficiency_bonus: raw.bonus_proficiency_bonus.as_deref().and_then(parse_bonus),
    };

    let charges = raw.charges.as_ref().and_then(value_display_text).map(|max| ItemCharges {
        max,
        recharge_trigger: raw.recharge.as_ref().map(|s| clean_tags(s)),
        recharge_amount: raw.recharge_amount.as_ref().and_then(value_display_text),
    });

    let mut spellcasting_focus_for = Vec::new();
    match &raw.focus {
        Some(Value::Bool(true)) => spellcasting_focus_for.push("Any".to_string()),
        Some(Value::Array(classes)) => {
            spellcasting_focus_for.extend(classes.iter().filter_map(Value::as_str).map(String::from))
        }
        _ => {}
    }
    if spellcasting_focus_for.is_empty() && matches!(&raw.staff, Some(Value::Bool(true))) {
        spellcasting_focus_for.push("Any".to_string());
    }

    let srd = matches!(&raw.srd, Some(Value::Bool(true)) | Some(Value::String(_)));

    let vehicle_extra = {
        let vehicle_fields: Map<String, Value> = raw
            .extra
            .iter()
            .filter(|(key, _)| {
                key.starts_with("veh")
                    || matches!(
                        key.as_str(),
                        "crew" | "crewMin" | "crewMax" | "capCargo" | "capPassenger" | "travelCost" | "shippingCost"
                    )
            })
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        if vehicle_fields.is_empty() { None } else { Some(Value::Object(vehicle_fields)) }
    };

    Item {
        id,
        canonical_id,
        name: raw.name,
        source: raw.source,
        is_group,
        group_members: if is_group {
            let hidden_refs = raw.items_hidden.as_ref().and_then(Value::as_array).into_iter().flatten();
            raw.items
                .iter()
                .map(String::as_str)
                .chain(hidden_refs.filter_map(Value::as_str))
                .map(parse_ref_string)
                .collect()
        } else {
            Vec::new()
        },
        member_of_groups: raw.group.iter().map(|s| parse_ref_string(s).name).collect(),
        item_type_code,
        item_type_label,
        rarity: raw.rarity.unwrap_or_else(|| "none".to_string()),
        weight_lb: raw.weight,
        value_cp: raw.value,
        requires_attunement,
        attunement_note,
        wondrous: raw.wondrous,
        sentient: raw.sentient,
        curse: raw.curse,
        weapon_category: raw.weapon_category.as_deref().and_then(WeaponCategory::from_code),
        properties,
        damage,
        base_ac: raw.ac,
        range,
        base_item_ref: raw.base_item,
        base_item_name,
        magic_bonuses,
        charges,
        spellcasting_focus_for,
        srd,
        vehicle_extra,
        entries: entries_from_values(&raw.entries),
    }
}

#[cfg(test)]
#[path = "transform_item_tests.rs"]
mod tests;
