use super::attribute_multiclass_choices;

fn pool(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

fn chosen(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

#[test]
fn assigns_disjoint_pools_directly() {
    let pools = vec![pool(&["fireball"]), pool(&["cure_wounds"])];
    let required = vec![1, 1];
    assert!(attribute_multiclass_choices(&chosen(&["fireball", "cure_wounds"]), &pools, &required));
}

#[test]
fn rejects_a_pick_that_matches_no_pool() {
    let pools = vec![pool(&["fireball"])];
    let required = vec![1];
    assert!(!attribute_multiclass_choices(&chosen(&["magic_missile"]), &pools, &required));
}

#[test]
fn rejects_wrong_total_count() {
    let pools = vec![pool(&["fireball", "magic_missile"])];
    let required = vec![1];
    assert!(!attribute_multiclass_choices(&chosen(&["fireball", "magic_missile"]), &pools, &required));
}

// Regression for a real greedy-first-fit failure: three classes A/B/C each need
// one pick, pools overlap (item1/item2 shared by A+B, item3 shared by B+C), and
// a valid assignment exists (A=item1, B=item2, C=item3) but first-fit in
// submission order [item3, item1, item2] used to send item3 to B and strand
// item2 with no free pool. A real classes example: a Wizard/Sorcerer/Warlock
// multiclass build whose spell lists overlap can hit this depending on pick order.
#[test]
fn finds_a_valid_assignment_when_pools_overlap_and_first_fit_would_fail() {
    let pools = vec![pool(&["item1", "item2"]), pool(&["item1", "item2", "item3"]), pool(&["item3"])];
    let required = vec![1, 1, 1];
    assert!(attribute_multiclass_choices(&chosen(&["item3", "item1", "item2"]), &pools, &required));
}

#[test]
fn rejects_when_no_valid_assignment_exists_despite_overlap() {
    // Counts match overall (2 chosen, 2 required), but both choices only fit
    // pool0 while pool1's one slot has no eligible pick — genuinely
    // unsatisfiable, not a matching bug.
    let pools = vec![pool(&["item1", "item2"]), pool(&["item3"])];
    let required = vec![1, 1];
    assert!(!attribute_multiclass_choices(&chosen(&["item1", "item2"]), &pools, &required));
}
