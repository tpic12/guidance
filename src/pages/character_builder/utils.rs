use crate::models::optional_feature::ResourceCost;
use crate::models::species::Species;
use std::collections::HashMap;

/// Assigns shared spell/cantrip picks back to whichever class's own pool+budget
/// they satisfy, via Kuhn's augmenting-path algorithm — a greedy first-fit can
/// reject a valid assignment when classes' pools overlap (see `character_builder_tests.rs`).
pub(super) fn attribute_multiclass_choices(chosen: &[String], pools: &[Vec<String>], required: &[usize]) -> bool {
    if chosen.len() != required.iter().sum::<usize>() {
        return false;
    }
    let mut pool_items: Vec<Vec<usize>> = vec![Vec::new(); pools.len()];
    for item in 0..chosen.len() {
        let mut visited = vec![false; pools.len()];
        if !try_assign_to_pool(item, chosen, pools, required, &mut pool_items, &mut visited) {
            return false;
        }
    }
    true
}

fn try_assign_to_pool(
    item: usize,
    chosen: &[String],
    pools: &[Vec<String>],
    required: &[usize],
    pool_items: &mut [Vec<usize>],
    visited: &mut [bool],
) -> bool {
    for i in 0..pools.len() {
        if visited[i] || !pools[i].contains(&chosen[item]) {
            continue;
        }
        visited[i] = true;
        if pool_items[i].len() < required[i] {
            pool_items[i].push(item);
            return true;
        }
        for k in 0..pool_items[i].len() {
            let displaced = pool_items[i][k];
            if try_assign_to_pool(displaced, chosen, pools, required, pool_items, visited) {
                pool_items[i][k] = item;
                return true;
            }
        }
    }
    false
}

pub(super) fn group_species_by_name(rows: Vec<Species>) -> Vec<(String, Vec<Species>)> {
    let mut groups: Vec<(String, Vec<Species>)> = Vec::new();
    let mut index_by_name: HashMap<String, usize> = HashMap::new();
    for species in rows {
        match index_by_name.get(&species.name) {
            Some(&idx) => groups[idx].1.push(species),
            None => {
                index_by_name.insert(species.name.clone(), groups.len());
                groups.push((species.name.clone(), vec![species]));
            }
        }
    }
    groups
}

pub(super) fn species_subtitle(species: &Species) -> String {
    [
        species.ability.clone().unwrap_or_default(),
        species.size.clone().unwrap_or_default(),
        species.speed.clone().unwrap_or_default(),
    ]
    .into_iter()
    .filter(|part| !part.is_empty())
    .collect::<Vec<_>>()
    .join(" · ")
}

// Mirrors OptionalFeatureDetailCard's amount formatting
// (pages/optional_features.rs), minus its "Costs: " prefix — this renders as
// an already-labeled subtitle line under the feature name.
pub(super) fn optional_feature_cost_label(cost: &ResourceCost) -> String {
    let amount = match (cost.amount, cost.amount_min, cost.amount_max) {
        (Some(amount), _, _) => format!("{amount} "),
        (None, Some(min), Some(max)) => format!("{min}-{max} "),
        (None, Some(min), None) => format!("{min}+ "),
        _ => String::new(),
    };
    format!("{amount}{}", cost.name)
}

#[cfg(test)]
#[path = "utils_tests.rs"]
mod tests;
