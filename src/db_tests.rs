use super::*;
use crate::models::background::BackgroundSort;
use crate::models::character::{AbilityBonusSource, AbilityMethod, AbilityScores, HpMethod};
use crate::models::feat::FeatSort;
use crate::models::item::{
    AttunementRequirement, DamageType, Item, ItemDamage, ItemMagicBonuses, ItemProperty, ItemQuery, ItemSort,
    WeaponCategory,
};
use crate::models::optional_feature::{class_requirement_labels, required_pacts, FeatureType, PrerequisiteOption};
use crate::models::skill::SkillGrant;
use crate::models::species::SpeciesSort;
use crate::models::spell::{CastingType, Components, Duration, Range, School};
use crate::models::user::Permission;

/// Single connection: each in-memory sqlite connection is its own database.
async fn test_pool() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("failed to open in-memory sqlite");
    sqlx::migrate!("./migrations").run(&pool).await.expect("migrations failed");
    pool
}

fn spell(name: &str, level: u8, school: School, source: &str) -> Spell {
    Spell {
        id: name.to_lowercase().replace(' ', "-"),
        canonical_id: format!("{name}|{source}"),
        name: name.to_string(),
        source: source.to_string(),
        level,
        school,
        casting_time: 1,
        casting_type: CastingType::Action,
        casting_condition: None,
        range: Range { kind: "self".to_string(), distance: None, unit: None },
        components: Components::default(),
        duration: Duration {
            kind: "instant".to_string(),
            amount: None,
            unit: None,
            concentration: false,
        },
        ritual: false,
        description: vec![],
        higher_level: vec![],
    }
}

async fn insert_spell(pool: &SqlitePool, spell: &Spell) {
    sqlx::query("INSERT INTO spells (id, name, level, school, source, concentration, ritual, data_json) VALUES (?,?,?,?,?,?,?,?)")
        .bind(&spell.id).bind(&spell.name).bind(spell.level).bind(spell.school.as_str())
        .bind(&spell.source)
        .bind(spell.duration.concentration).bind(spell.ritual)
        .bind(serde_json::to_string(spell).unwrap())
        .execute(pool).await.unwrap();
}

async fn spell_fixtures(pool: &SqlitePool) {
    for s in [
        spell("Fireball", 3, School::Evocation, "PHB"),
        spell("Fire Bolt", 0, School::Evocation, "PHB"),
        spell("Charm Person", 1, School::Enchantment, "XGE"),
    ] {
        insert_spell(pool, &s).await;
    }
}

fn background(name: &str, source: &str) -> Background {
    Background {
        id: name.to_lowercase().replace(' ', "-"),
        canonical_id: format!("{name}|{source}"),
        name: name.to_string(),
        source: source.to_string(),
        skills: vec![SkillGrant::Fixed { skills: vec!["insight".to_string()] }],
        languages: vec![],
        tools: vec![],
        entries: vec![],
    }
}

fn feat(name: &str, source: &str, prerequisite: Option<&str>) -> Feat {
    Feat {
        id: name.to_lowercase().replace(' ', "-"),
        canonical_id: format!("{name}|{source}"),
        name: name.to_string(),
        source: source.to_string(),
        prerequisite: prerequisite.map(str::to_string),
        ability: None,
        entries: vec![],
    }
}

fn species(name: &str, source: &str, ability: Option<&str>) -> Species {
    Species {
        id: name.to_lowercase().replace(' ', "-"),
        canonical_id: format!("{name}|{source}"),
        name: name.to_string(),
        source: source.to_string(),
        ability: ability.map(str::to_string),
        ability_bonuses: vec![],
        size: Some("Medium".to_string()),
        speed: Some("30 ft.".to_string()),
        darkvision: None,
        entries: vec![],
    }
}

async fn insert_flat(pool: &SqlitePool, table: &str, id: &str, name: &str, source: &str, data_json: String) {
    sqlx::query(&format!("INSERT INTO {table} (id, name, source, data_json) VALUES (?,?,?,?)"))
        .bind(id).bind(name).bind(source).bind(data_json)
        .execute(pool).await.unwrap();
}

#[tokio::test]
async fn list_spells_unfiltered_sorts_by_name() {
    let pool = test_pool().await;
    spell_fixtures(&pool).await;

    let names: Vec<String> = list_spells(&pool, &SpellQuery::default())
        .await
        .unwrap()
        .into_iter()
        .map(|s| s.name)
        .collect();
    assert_eq!(names, ["Charm Person", "Fire Bolt", "Fireball"]);
}

#[tokio::test]
async fn list_spells_combines_filters() {
    let pool = test_pool().await;
    spell_fixtures(&pool).await;

    let query = SpellQuery {
        search: "fire".to_string(),
        schools: vec![School::Evocation],
        levels: vec![3],
        sources: vec!["PHB".to_string()],
        classes: vec![],
        sort: SpellSort::default(),
    };
    let spells = list_spells(&pool, &query).await.unwrap();
    assert_eq!(spells.len(), 1);
    assert_eq!(spells[0].name, "Fireball");
    // The row round-trips the full record, not just the filter columns.
    assert_eq!(spells[0].canonical_id, "Fireball|PHB");
}

#[tokio::test]
async fn list_spells_filters_by_linked_class() {
    let pool = test_pool().await;
    spell_fixtures(&pool).await; // Fireball(3), Fire Bolt(0), Charm Person(1)
    insert_class(&pool, "fake-mage", "Fake Mage").await;
    insert_class(&pool, "fake-warrior", "Fake Warrior").await;
    for (class_id, spell_id) in [("fake-mage", "fireball"), ("fake-mage", "fire-bolt"), ("fake-warrior", "fireball")]
    {
        sqlx::query("INSERT INTO class_spells (class_id, spell_id) VALUES (?, ?)")
            .bind(class_id)
            .bind(spell_id)
            .execute(&pool)
            .await
            .unwrap();
    }

    let query = SpellQuery { classes: vec!["Fake Warrior".to_string()], ..SpellQuery::default() };
    let names: Vec<String> =
        list_spells(&pool, &query).await.unwrap().into_iter().map(|s| s.name).collect();
    assert_eq!(names, ["Fireball"]);

    // Charm Person isn't linked to either class, so it's excluded once any
    // class filter is applied, even though a broader filter (both classes)
    // still dedupes Fireball rather than doubling it up.
    let query =
        SpellQuery { classes: vec!["Fake Mage".to_string(), "Fake Warrior".to_string()], ..SpellQuery::default() };
    let names: Vec<String> =
        list_spells(&pool, &query).await.unwrap().into_iter().map(|s| s.name).collect();
    assert_eq!(names, ["Fire Bolt", "Fireball"]);
}

#[tokio::test]
async fn list_spell_classes_lists_distinct_linked_class_names() {
    let pool = test_pool().await;
    spell_fixtures(&pool).await;
    insert_class(&pool, "fake-mage", "Fake Mage").await;
    insert_class(&pool, "fake-warrior", "Fake Warrior").await;
    // Fake Warrior has no spells linked at all — shouldn't show up.
    sqlx::query("INSERT INTO class_spells (class_id, spell_id) VALUES (?, ?)")
        .bind("fake-mage")
        .bind("fireball")
        .execute(&pool)
        .await
        .unwrap();

    let classes = list_spell_classes(&pool).await.unwrap();
    assert_eq!(classes, vec!["Fake Mage"]);
}

#[tokio::test]
async fn list_spells_level_sort_breaks_ties_by_name() {
    let pool = test_pool().await;
    spell_fixtures(&pool).await;

    let query = SpellQuery { sort: SpellSort::LevelDesc, ..SpellQuery::default() };
    let names: Vec<String> =
        list_spells(&pool, &query).await.unwrap().into_iter().map(|s| s.name).collect();
    assert_eq!(names, ["Fireball", "Charm Person", "Fire Bolt"]);
}

#[tokio::test]
async fn list_spell_sources_is_distinct_and_sorted() {
    let pool = test_pool().await;
    spell_fixtures(&pool).await;

    assert_eq!(list_spell_sources(&pool).await.unwrap(), ["PHB", "XGE"]);
}

#[tokio::test]
async fn list_backgrounds_filters_and_sorts() {
    let pool = test_pool().await;
    for b in [background("Acolyte", "PHB"), background("Urchin", "PHB"), background("Anthropologist", "ToA")] {
        insert_flat(&pool, "backgrounds", &b.id, &b.name, &b.source, serde_json::to_string(&b).unwrap()).await;
    }

    let all = list_backgrounds(&pool, &BackgroundQuery::default()).await.unwrap();
    assert_eq!(all.len(), 3);
    assert_eq!(all[0].skills, [SkillGrant::Fixed { skills: vec!["insight".to_string()] }]);

    let query = BackgroundQuery {
        search: " aco ".to_string(), // whitespace is trimmed before matching
        sources: vec!["PHB".to_string()],
        sort: BackgroundSort::default(),
    };
    let filtered = list_backgrounds(&pool, &query).await.unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].name, "Acolyte");

    let query = BackgroundQuery { sort: BackgroundSort::SourceDesc, ..BackgroundQuery::default() };
    let first = list_backgrounds(&pool, &query).await.unwrap().remove(0);
    assert_eq!(first.source, "ToA");

    assert_eq!(list_background_sources(&pool).await.unwrap(), ["PHB", "ToA"]);
}

#[tokio::test]
async fn list_feats_round_trips_prerequisites() {
    let pool = test_pool().await;
    for f in [feat("Grappler", "PHB", Some("Strength 13 or higher")), feat("Alert", "PHB", None)] {
        insert_flat(&pool, "feats", &f.id, &f.name, &f.source, serde_json::to_string(&f).unwrap()).await;
    }

    let query = FeatQuery { sort: FeatSort::NameDesc, ..FeatQuery::default() };
    let feats = list_feats(&pool, &query).await.unwrap();
    assert_eq!(feats.len(), 2);
    assert_eq!(feats[0].name, "Grappler");
    assert_eq!(feats[0].prerequisite.as_deref(), Some("Strength 13 or higher"));
    assert_eq!(feats[1].prerequisite, None);

    assert_eq!(list_feat_sources(&pool).await.unwrap(), ["PHB"]);
}

fn optional_feature(name: &str, source: &str, feature_types: Vec<FeatureType>) -> OptionalFeature {
    OptionalFeature {
        id: name.to_lowercase().replace(' ', "-"),
        canonical_id: format!("{name}|{source}"),
        name: name.to_string(),
        source: source.to_string(),
        feature_types,
        prerequisites: vec![],
        consumes: None,
        entries: vec![],
    }
}

async fn insert_optional_feature(pool: &SqlitePool, feature: &OptionalFeature) {
    insert_flat(pool, "optional_features", &feature.id, &feature.name, &feature.source, serde_json::to_string(feature).unwrap()).await;
    for feature_type in &feature.feature_types {
        sqlx::query("INSERT INTO optional_feature_types (optional_feature_id, feature_type) VALUES (?, ?)")
            .bind(&feature.id)
            .bind(feature_type.as_str())
            .execute(pool)
            .await
            .unwrap();
    }
    for class_requirement in class_requirement_labels(&feature.prerequisites) {
        sqlx::query(
            "INSERT INTO optional_feature_prerequisite_classes (optional_feature_id, class_requirement) VALUES (?, ?)",
        )
        .bind(&feature.id)
        .bind(class_requirement)
        .execute(pool)
        .await
        .unwrap();
    }
    for pact in required_pacts(&feature.prerequisites) {
        sqlx::query("INSERT INTO optional_feature_prerequisite_pacts (optional_feature_id, pact) VALUES (?, ?)")
            .bind(&feature.id)
            .bind(pact)
            .execute(pool)
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn list_optional_features_filters_by_type_source_and_search() {
    let pool = test_pool().await;
    for f in [
        optional_feature("Agonizing Blast", "PHB", vec![FeatureType::EldritchInvocation]),
        optional_feature("Careful Spell", "PHB", vec![FeatureType::Metamagic]),
        optional_feature("Archery", "XGE", vec![FeatureType::FightingStyleFighter, FeatureType::FightingStyleRanger]),
    ] {
        insert_optional_feature(&pool, &f).await;
    }

    let all = list_optional_features(&pool, &OptionalFeatureQuery::default()).await.unwrap();
    assert_eq!(all.len(), 3);

    let by_type = list_optional_features(
        &pool,
        &OptionalFeatureQuery { types: vec![FeatureType::FightingStyleRanger], ..Default::default() },
    )
    .await
    .unwrap();
    assert_eq!(by_type.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(), vec!["Archery"]);

    let by_source = list_optional_features(
        &pool,
        &OptionalFeatureQuery { sources: vec!["PHB".to_string()], ..Default::default() },
    )
    .await
    .unwrap();
    assert_eq!(by_source.len(), 2);

    let by_search = list_optional_features(
        &pool,
        &OptionalFeatureQuery { search: "agonizing".to_string(), ..Default::default() },
    )
    .await
    .unwrap();
    assert_eq!(by_search.len(), 1);
    assert_eq!(by_search[0].name, "Agonizing Blast");

    assert_eq!(list_optional_feature_sources(&pool).await.unwrap(), ["PHB", "XGE"]);
}

#[tokio::test]
async fn list_optional_features_filters_by_class_requirement_and_pact() {
    let pool = test_pool().await;
    let mut ascendant_step =
        optional_feature("Ascendant Step", "PHB", vec![FeatureType::EldritchInvocation]);
    ascendant_step.prerequisites = vec![PrerequisiteOption {
        level: Some(9),
        class_name: Some("Warlock".to_string()),
        ..Default::default()
    }];

    let mut thirsting_blade =
        optional_feature("Thirsting Blade", "PHB", vec![FeatureType::EldritchInvocation]);
    thirsting_blade.prerequisites = vec![PrerequisiteOption {
        level: Some(5),
        class_name: Some("Warlock".to_string()),
        pact: Some("Blade".to_string()),
        ..Default::default()
    }];

    let mut breath_of_winter =
        optional_feature("Breath of Winter", "PHB", vec![FeatureType::ElementalDiscipline]);
    breath_of_winter.prerequisites = vec![PrerequisiteOption {
        level: Some(17),
        class_name: Some("Monk".to_string()),
        subclass_name: Some("Four Elements".to_string()),
        ..Default::default()
    }];

    for f in [&ascendant_step, &thirsting_blade, &breath_of_winter] {
        insert_optional_feature(&pool, f).await;
    }

    let by_class = list_optional_features(
        &pool,
        &OptionalFeatureQuery { class_requirements: vec!["Warlock".to_string()], ..Default::default() },
    )
    .await
    .unwrap();
    assert_eq!(
        by_class.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(),
        vec!["Ascendant Step", "Thirsting Blade"]
    );

    let by_subclass = list_optional_features(
        &pool,
        &OptionalFeatureQuery {
            class_requirements: vec!["Monk (Four Elements)".to_string()],
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(by_subclass.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(), vec!["Breath of Winter"]);

    let by_pact = list_optional_features(
        &pool,
        &OptionalFeatureQuery { pacts: vec!["Blade".to_string()], ..Default::default() },
    )
    .await
    .unwrap();
    assert_eq!(by_pact.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(), vec!["Thirsting Blade"]);

    assert_eq!(
        list_optional_feature_class_requirements(&pool).await.unwrap(),
        ["Monk (Four Elements)", "Warlock"]
    );
    assert_eq!(list_optional_feature_pacts(&pool).await.unwrap(), ["Blade"]);
}

#[tokio::test]
async fn list_optional_features_sorts_by_name_and_source() {
    let pool = test_pool().await;
    for f in [
        optional_feature("Zeal", "PHB", vec![FeatureType::PactBoon]),
        optional_feature("Armor", "XGE", vec![FeatureType::PactBoon]),
    ] {
        insert_optional_feature(&pool, &f).await;
    }

    let query = OptionalFeatureQuery { sort: OptionalFeatureSort::NameAsc, ..Default::default() };
    let names: Vec<String> =
        list_optional_features(&pool, &query).await.unwrap().into_iter().map(|f| f.name).collect();
    assert_eq!(names, ["Armor", "Zeal"]);

    let query = OptionalFeatureQuery { sort: OptionalFeatureSort::SourceDesc, ..Default::default() };
    let first = list_optional_features(&pool, &query).await.unwrap().remove(0);
    assert_eq!(first.source, "XGE");
}

#[tokio::test]
async fn list_species_filters_and_round_trips() {
    let pool = test_pool().await;
    for s in [
        species("Elf", "PHB", Some("Dexterity +2")),
        species("Dwarf", "PHB", Some("Constitution +2")),
        species("Warforged", "ERLW", None),
    ] {
        insert_flat(&pool, "species", &s.id, &s.name, &s.source, serde_json::to_string(&s).unwrap()).await;
    }

    let all = list_species(&pool, &SpeciesQuery::default()).await.unwrap();
    assert_eq!(all.len(), 3);
    assert_eq!(all[0].name, "Dwarf");
    assert_eq!(all[0].ability.as_deref(), Some("Constitution +2"));
    assert_eq!(all[0].size.as_deref(), Some("Medium"));

    let query = SpeciesQuery {
        search: "elf".to_string(),
        sources: vec!["PHB".to_string()],
        sort: SpeciesSort::default(),
    };
    let filtered = list_species(&pool, &query).await.unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].name, "Elf");

    let query = SpeciesQuery { sort: SpeciesSort::SourceDesc, ..SpeciesQuery::default() };
    let first = list_species(&pool, &query).await.unwrap().remove(0);
    assert_eq!(first.source, "PHB");

    assert_eq!(list_species_sources(&pool).await.unwrap(), ["ERLW", "PHB"]);
}

fn item(name: &str, source: &str) -> Item {
    Item {
        id: name.to_lowercase().replace(' ', "-"),
        canonical_id: format!("{name}|{source}"),
        name: name.to_string(),
        source: source.to_string(),
        is_group: false,
        group_members: vec![],
        member_of_groups: vec![],
        item_type_code: None,
        item_type_label: None,
        rarity: "none".to_string(),
        weight_lb: None,
        value_cp: None,
        requires_attunement: false,
        attunement_note: None,
        wondrous: false,
        sentient: false,
        curse: false,
        weapon_category: None,
        properties: vec![],
        damage: None,
        base_ac: None,
        range: None,
        base_item_ref: None,
        base_item_name: None,
        magic_bonuses: ItemMagicBonuses::default(),
        charges: None,
        spellcasting_focus_for: vec![],
        srd: false,
        vehicle_extra: None,
        entries: vec![],
    }
}

async fn insert_item(pool: &SqlitePool, item: &Item) {
    sqlx::query(
        "INSERT INTO items (id, name, source, is_group, item_type, rarity, rarity_rank, \
         requires_attunement, weapon_category, damage_type, value_cp, weight_lb, data_json) \
         VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)",
    )
    .bind(&item.id)
    .bind(&item.name)
    .bind(&item.source)
    .bind(item.is_group)
    .bind(&item.item_type_label)
    .bind(&item.rarity)
    .bind(crate::models::item::rarity_rank(&item.rarity))
    .bind(item.requires_attunement)
    .bind(item.weapon_category.map(|category| category.as_str()))
    .bind(item.damage.as_ref().and_then(|damage| damage.damage_type).map(|d| d.as_str()))
    .bind(item.value_cp)
    .bind(item.weight_lb)
    .bind(serde_json::to_string(item).unwrap())
    .execute(pool)
    .await
    .unwrap();

    for property in &item.properties {
        sqlx::query("INSERT INTO item_properties (item_id, property_code, property_label) VALUES (?,?,?)")
            .bind(&item.id)
            .bind(&property.code)
            .bind(&property.label)
            .execute(pool)
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn list_items_combines_all_filter_dimensions() {
    let pool = test_pool().await;

    let blade = Item {
        item_type_label: Some("Melee Weapon".to_string()),
        rarity: "rare".to_string(),
        weight_lb: Some(3.0),
        value_cp: Some(1500.0),
        requires_attunement: true,
        weapon_category: Some(WeaponCategory::Simple),
        properties: vec![
            ItemProperty { code: "F".to_string(), label: "Finesse".to_string() },
            ItemProperty { code: "L".to_string(), label: "Light".to_string() },
        ],
        damage: Some(ItemDamage {
            dice: "1d8".to_string(),
            dice_versatile: None,
            damage_type: Some(DamageType::Slashing),
        }),
        ..item("Fireheart Blade", "PHB")
    };
    let buckler = Item {
        item_type_label: Some("Shield".to_string()),
        rarity: "none".to_string(),
        weight_lb: Some(6.0),
        value_cp: Some(1000.0),
        ..item("Sturdy Buckler", "PHB")
    };
    let orb = Item {
        item_type_label: Some("Wondrous Item".to_string()),
        rarity: "legendary".to_string(),
        requires_attunement: true,
        ..item("Orb of Mystery", "XGE")
    };
    for i in [&blade, &buckler, &orb] {
        insert_item(&pool, i).await;
    }

    // Every dimension at once, all pointing at the same single record.
    let query = ItemQuery {
        search: "fireheart".to_string(),
        sources: vec!["PHB".to_string()],
        types: vec!["Melee Weapon".to_string()],
        rarities: vec!["rare".to_string()],
        property_codes: vec!["F".to_string()],
        weapon_categories: vec![WeaponCategory::Simple],
        damage_types: vec![DamageType::Slashing],
        attunement: vec![AttunementRequirement::Required],
        ..Default::default()
    };
    let matches = list_items(&pool, &query).await.unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].name, "Fireheart Blade");
    // The row round-trips the full record, not just the filter columns.
    assert_eq!(matches[0].damage.as_ref().unwrap().dice, "1d8");

    // Same query but for a rarity Fireheart Blade doesn't have — filters
    // must combine with AND, not each apply independently.
    let query = ItemQuery { rarities: vec!["legendary".to_string()], ..query };
    assert!(list_items(&pool, &query).await.unwrap().is_empty());

    // A property Fireheart Blade doesn't carry, isolated from the rest.
    let by_property =
        list_items(&pool, &ItemQuery { property_codes: vec!["L".to_string()], ..Default::default() })
            .await
            .unwrap();
    assert_eq!(by_property.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(), vec!["Fireheart Blade"]);
}

#[tokio::test]
async fn list_items_rarity_sort_orders_by_severity_not_alphabetically() {
    let pool = test_pool().await;
    // Alphabetically "artifact" < "none" < "rare", the opposite of severity.
    for i in [
        Item { rarity: "rare".to_string(), ..item("Fireheart Blade", "PHB") },
        Item { rarity: "none".to_string(), ..item("Sturdy Buckler", "PHB") },
        Item { rarity: "artifact".to_string(), ..item("Cloak of Many Colors", "XGE") },
    ] {
        insert_item(&pool, &i).await;
    }

    let ascending = list_items(&pool, &ItemQuery { sort: ItemSort::RarityAsc, ..Default::default() }).await.unwrap();
    assert_eq!(
        ascending.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(),
        vec!["Sturdy Buckler", "Fireheart Blade", "Cloak of Many Colors"]
    );

    let descending =
        list_items(&pool, &ItemQuery { sort: ItemSort::RarityDesc, ..Default::default() }).await.unwrap();
    assert_eq!(
        descending.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(),
        vec!["Cloak of Many Colors", "Fireheart Blade", "Sturdy Buckler"]
    );

    assert_eq!(list_item_rarities(&pool).await.unwrap(), ["none", "rare", "artifact"]);
}

#[tokio::test]
async fn list_items_value_and_weight_sorts_put_nulls_last_either_direction() {
    let pool = test_pool().await;
    for i in [
        Item { value_cp: Some(1500.0), weight_lb: Some(3.0), ..item("Fireheart Blade", "PHB") },
        Item { value_cp: Some(1000.0), weight_lb: Some(6.0), ..item("Sturdy Buckler", "PHB") },
        // No listed cost or weight — must sort last regardless of direction.
        Item { value_cp: None, weight_lb: None, ..item("Orb of Mystery", "XGE") },
    ] {
        insert_item(&pool, &i).await;
    }

    let value_asc = list_items(&pool, &ItemQuery { sort: ItemSort::ValueAsc, ..Default::default() }).await.unwrap();
    assert_eq!(
        value_asc.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(),
        vec!["Sturdy Buckler", "Fireheart Blade", "Orb of Mystery"]
    );

    let value_desc = list_items(&pool, &ItemQuery { sort: ItemSort::ValueDesc, ..Default::default() }).await.unwrap();
    assert_eq!(
        value_desc.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(),
        vec!["Fireheart Blade", "Sturdy Buckler", "Orb of Mystery"]
    );

    let weight_asc = list_items(&pool, &ItemQuery { sort: ItemSort::WeightAsc, ..Default::default() }).await.unwrap();
    assert_eq!(
        weight_asc.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(),
        vec!["Fireheart Blade", "Sturdy Buckler", "Orb of Mystery"]
    );

    let weight_desc =
        list_items(&pool, &ItemQuery { sort: ItemSort::WeightDesc, ..Default::default() }).await.unwrap();
    assert_eq!(
        weight_desc.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(),
        vec!["Sturdy Buckler", "Fireheart Blade", "Orb of Mystery"]
    );
}

#[tokio::test]
async fn list_items_source_sort_breaks_ties_by_name() {
    let pool = test_pool().await;
    for i in [
        item("Sturdy Buckler", "PHB"),
        item("Fireheart Blade", "PHB"),
        item("Orb of Mystery", "XGE"),
    ] {
        insert_item(&pool, &i).await;
    }

    let ascending = list_items(&pool, &ItemQuery { sort: ItemSort::SourceAsc, ..Default::default() }).await.unwrap();
    assert_eq!(
        ascending.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(),
        vec!["Fireheart Blade", "Sturdy Buckler", "Orb of Mystery"]
    );

    let descending =
        list_items(&pool, &ItemQuery { sort: ItemSort::SourceDesc, ..Default::default() }).await.unwrap();
    assert_eq!(descending[0].source, "XGE");
    assert_eq!(descending[0].name, "Orb of Mystery");

    assert_eq!(list_item_sources(&pool).await.unwrap(), ["PHB", "XGE"]);
}

#[tokio::test]
async fn list_item_lookup_helpers_are_distinct_and_correctly_ordered() {
    let pool = test_pool().await;
    let blade = Item {
        item_type_label: Some("Melee Weapon".to_string()),
        properties: vec![
            ItemProperty { code: "F".to_string(), label: "Finesse".to_string() },
            ItemProperty { code: "L".to_string(), label: "Light".to_string() },
        ],
        ..item("Fireheart Blade", "PHB")
    };
    let dagger = Item {
        item_type_label: Some("Melee Weapon".to_string()),
        properties: vec![ItemProperty { code: "L".to_string(), label: "Light".to_string() }],
        ..item("Quiet Dagger", "PHB")
    };
    for i in [&blade, &dagger] {
        insert_item(&pool, i).await;
    }

    assert_eq!(list_item_types(&pool).await.unwrap(), ["Melee Weapon"]);

    let properties = list_item_properties(&pool).await.unwrap();
    assert_eq!(properties.iter().map(|p| p.code.as_str()).collect::<Vec<_>>(), vec!["F", "L"]);
    assert_eq!(properties.iter().map(|p| p.label.as_str()).collect::<Vec<_>>(), vec!["Finesse", "Light"]);
}

#[tokio::test]
async fn get_item_round_trips_and_missing_id_is_none() {
    let pool = test_pool().await;
    insert_item(&pool, &item("Fireheart Blade", "PHB")).await;

    let found = get_item(&pool, "fireheart-blade").await.unwrap().unwrap();
    assert_eq!(found.name, "Fireheart Blade");
    assert_eq!(found.canonical_id, "Fireheart Blade|PHB");

    assert!(get_item(&pool, "nope").await.unwrap().is_none());
}

fn character(id: &str, name: &str) -> Character {
    Character {
        id: id.to_string(),
        user_id: "owner-1".to_string(),
        name: name.to_string(),
        species_id: "elf".to_string(),
        background_id: "acolyte".to_string(),
        classes: vec![crate::models::character::ClassLevel {
            class_id: "fake-warrior".to_string(),
            subclass_id: None,
            level: 5,
        }],
        ability_method: AbilityMethod::Manual,
        abilities: AbilityScores::default(),
        asi_choices: vec![Some(AsiChoice::Feat { feat_id: "grappler".to_string() })],
        skill_choices: vec![],
        expertise_choices: vec![],
        cantrip_choices: vec![],
        spell_choices: vec![],
        spell_grant_choices: vec![],
        optional_feature_choices: vec![],
        species_ability_choices: vec![],
        ability_bonus_source: AbilityBonusSource::default(),
        custom_ability_bonus_choices: vec![],
        hp_method: HpMethod::default(),
        hp_rolls: vec![],
        hp_manual: None,
        enforce_multiclass_prereqs: true,
    }
}

async fn insert_user(pool: &SqlitePool, id: &str, username: &str) {
    sqlx::query("INSERT INTO users (id, username, password_hash) VALUES (?, ?, '')")
        .bind(id)
        .bind(username)
        .execute(pool)
        .await
        .unwrap();
}

async fn insert_class(pool: &SqlitePool, id: &str, name: &str) {
    let class = crate::models::class::Class {
        id: id.to_string(),
        canonical_id: format!("{name}|TBK"),
        name: name.to_string(),
        source: "TBK".to_string(),
        hit_die: 10,
        saving_throws: vec!["str".to_string(), "con".to_string()],
        subclass_title: "Fighting Style".to_string(),
        caster_progression: None,
        spellcasting_ability: None,
        spells_known_progression: None,
        cantrips_known_progression: None,
        proficiencies: Default::default(),
        multiclass_proficiencies: Default::default(),
        starting_equipment: vec![],
        table_groups: vec![],
        optional_feature_progressions: vec![],
        multiclass_ability_prerequisites: vec![],
    };
    sqlx::query("INSERT INTO classes (id, name, source, hit_die, data_json) VALUES (?,?,?,?,?)")
        .bind(id).bind(name).bind("TBK").bind(10)
        .bind(serde_json::to_string(&class).unwrap())
        .execute(pool).await.unwrap();
}

async fn insert_subclass(pool: &SqlitePool, id: &str, class_id: &str, name: &str) {
    let subclass = crate::models::class::Subclass {
        id: id.to_string(),
        class_id: class_id.to_string(),
        name: name.to_string(),
        short_name: name.to_string(),
        source: "TBK".to_string(),
        caster_progression: None,
        spellcasting_ability: None,
        spells_known_progression: None,
        cantrips_known_progression: None,
        optional_feature_progressions: vec![],
    };
    sqlx::query("INSERT INTO subclasses (id, class_id, name, short_name, source, data_json) VALUES (?,?,?,?,?,?)")
        .bind(id).bind(class_id).bind(name).bind(name).bind("TBK")
        .bind(serde_json::to_string(&subclass).unwrap())
        .execute(pool).await.unwrap();
}

#[tokio::test]
async fn list_spells_for_class_includes_subclass_linked_spells_and_dedupes() {
    let pool = test_pool().await;
    spell_fixtures(&pool).await; // Fireball(3), Fire Bolt(0), Charm Person(1)
    insert_class(&pool, "fake-fighter", "Fake Fighter").await;
    insert_subclass(&pool, "fake-eldritch-knight", "fake-fighter", "Fake Eldritch Knight").await;

    sqlx::query("INSERT INTO class_spells (class_id, spell_id) VALUES (?, ?)")
        .bind("fake-fighter")
        .bind("fireball")
        .execute(&pool)
        .await
        .unwrap();
    // Charm Person only reachable via the subclass; Fireball reachable via
    // both, to exercise the union's dedup.
    for spell_id in ["charm-person", "fireball"] {
        sqlx::query("INSERT INTO subclass_spells (subclass_id, spell_id) VALUES (?, ?)")
            .bind("fake-eldritch-knight")
            .bind(spell_id)
            .execute(&pool)
            .await
            .unwrap();
    }

    let class_only = list_spells_for_class(&pool, "fake-fighter", None, 20, 9).await.unwrap();
    assert_eq!(class_only.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), vec!["Fireball"]);

    let with_subclass =
        list_spells_for_class(&pool, "fake-fighter", Some("fake-eldritch-knight"), 20, 9).await.unwrap();
    assert_eq!(
        with_subclass.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(),
        vec!["Charm Person", "Fireball"]
    );

    let capped =
        list_spells_for_class(&pool, "fake-fighter", Some("fake-eldritch-knight"), 20, 1).await.unwrap();
    assert_eq!(capped.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), vec!["Charm Person"]);
}

#[tokio::test]
async fn list_spells_for_class_excludes_subclass_granted_spells_once_unlocked() {
    let pool = test_pool().await;
    spell_fixtures(&pool).await; // Fireball(3), Fire Bolt(0), Charm Person(1)
    insert_class(&pool, "fake-cleric", "Fake Cleric").await;
    insert_subclass(&pool, "fake-life-domain", "fake-cleric", "Fake Life Domain").await;

    sqlx::query("INSERT INTO subclass_spells (subclass_id, spell_id) VALUES (?, ?)")
        .bind("fake-life-domain")
        .bind("fireball")
        .execute(&pool)
        .await
        .unwrap();
    // Fireball is also auto-granted (always prepared) starting level 3 — it
    // should disappear from the pickable pool once the character is high
    // enough level to have received the grant, but not before.
    sqlx::query(
        "INSERT INTO subclass_granted_spells (subclass_id, spell_id, grant_level) VALUES (?, ?, ?)",
    )
    .bind("fake-life-domain")
    .bind("fireball")
    .bind(3_i64)
    .execute(&pool)
    .await
    .unwrap();

    let before_grant =
        list_spells_for_class(&pool, "fake-cleric", Some("fake-life-domain"), 2, 9).await.unwrap();
    assert_eq!(before_grant.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), vec!["Fireball"]);

    let after_grant =
        list_spells_for_class(&pool, "fake-cleric", Some("fake-life-domain"), 3, 9).await.unwrap();
    assert!(after_grant.is_empty());
}

#[tokio::test]
async fn get_subclass_granted_spells_returns_only_unlocked_grants_ordered_by_level() {
    let pool = test_pool().await;
    spell_fixtures(&pool).await; // Fireball(3), Fire Bolt(0), Charm Person(1)
    insert_class(&pool, "fake-cleric", "Fake Cleric").await;
    insert_subclass(&pool, "fake-life-domain", "fake-cleric", "Fake Life Domain").await;

    for (spell_id, level) in [("charm-person", 1_i64), ("fireball", 3_i64)] {
        sqlx::query(
            "INSERT INTO subclass_granted_spells (subclass_id, spell_id, grant_level) VALUES (?, ?, ?)",
        )
        .bind("fake-life-domain")
        .bind(spell_id)
        .bind(level)
        .execute(&pool)
        .await
        .unwrap();
    }

    let at_level_1 = get_subclass_granted_spells(&pool, "fake-life-domain", 1).await.unwrap();
    assert_eq!(at_level_1.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), vec!["Charm Person"]);

    let at_level_3 = get_subclass_granted_spells(&pool, "fake-life-domain", 3).await.unwrap();
    assert_eq!(
        at_level_3.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(),
        vec!["Charm Person", "Fireball"]
    );
}

#[tokio::test]
async fn get_subclass_spell_choice_pools_filters_by_level_class_school_and_gates_on_grant_level() {
    // Mirrors Cleric Nature Domain (`class=Druid`) and Death Domain
    // (`school=N`... here Evocation for the fixture spells we have) — see
    // Vikunja #57.
    let pool = test_pool().await;
    spell_fixtures(&pool).await; // Fireball(3, Evocation), Fire Bolt(0, Evocation), Charm Person(1, Enchantment)
    insert_class(&pool, "fake-cleric", "Fake Cleric").await;
    insert_class(&pool, "fake-druid", "Fake Druid").await;
    insert_subclass(&pool, "fake-nature-domain", "fake-cleric", "Fake Nature Domain").await;

    sqlx::query("INSERT INTO class_spells (class_id, spell_id) VALUES (?, ?)")
        .bind("fake-druid")
        .bind("fire-bolt")
        .execute(&pool)
        .await
        .unwrap();

    // A `class=Druid` cantrip filter, unlocked at level 1.
    sqlx::query(
        "INSERT INTO subclass_spell_choice_grants (subclass_id, grant_level, spell_level, class_name, school, count) VALUES (?,?,?,?,?,?)",
    )
    .bind("fake-nature-domain")
    .bind(1_i64)
    .bind(0_i64)
    .bind("Fake Druid")
    .bind(None::<String>)
    .bind(1_i64)
    .execute(&pool)
    .await
    .unwrap();
    // A `school=Evocation`, level-3 filter (no class restriction), unlocked
    // only at level 5, with a count of 2.
    sqlx::query(
        "INSERT INTO subclass_spell_choice_grants (subclass_id, grant_level, spell_level, class_name, school, count) VALUES (?,?,?,?,?,?)",
    )
    .bind("fake-nature-domain")
    .bind(5_i64)
    .bind(3_i64)
    .bind(None::<String>)
    .bind("evocation")
    .bind(2_i64)
    .execute(&pool)
    .await
    .unwrap();

    let at_level_1 = get_subclass_spell_choice_pools(&pool, "fake-nature-domain", 1).await.unwrap();
    assert_eq!(at_level_1.len(), 1);
    assert_eq!(at_level_1[0].0, 1);
    assert_eq!(at_level_1[0].1.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), vec!["Fire Bolt"]);

    let at_level_5 = get_subclass_spell_choice_pools(&pool, "fake-nature-domain", 5).await.unwrap();
    assert_eq!(at_level_5.len(), 2);
    let evocation_pool = at_level_5.iter().find(|(count, _)| *count == 2).unwrap();
    assert_eq!(evocation_pool.1.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), vec!["Fireball"]);
}

#[tokio::test]
async fn subclass_spell_choice_grants_insert_or_ignore_dedupes_on_reseed() {
    // A `SEED_RESET=1` reseed re-inserts every grant row `seed_classes` reads
    // from the fixtures. Without a real uniqueness constraint that treats
    // NULL `class_name`/`school` consistently (SQLite's plain UNIQUE treats
    // every NULL as distinct from every other NULL), re-seeding would
    // silently duplicate rows for the common case where only one of those
    // two filter fields is set. Exercise both the null-school and
    // null-class_name shapes to confirm the COALESCE'd unique index catches
    // both, not just the case where every column happens to be non-null.
    let pool = test_pool().await;
    insert_class(&pool, "fake-cleric", "Fake Cleric").await;
    insert_subclass(&pool, "fake-nature-domain", "fake-cleric", "Fake Nature Domain").await;

    for _ in 0..2 {
        sqlx::query(
            "INSERT OR IGNORE INTO subclass_spell_choice_grants (subclass_id, grant_level, spell_level, class_name, school, count) VALUES (?,?,?,?,?,?)",
        )
        .bind("fake-nature-domain")
        .bind(1_i64)
        .bind(0_i64)
        .bind("Fake Druid")
        .bind(None::<String>)
        .bind(1_i64)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT OR IGNORE INTO subclass_spell_choice_grants (subclass_id, grant_level, spell_level, class_name, school, count) VALUES (?,?,?,?,?,?)",
        )
        .bind("fake-nature-domain")
        .bind(1_i64)
        .bind(0_i64)
        .bind(None::<String>)
        .bind("necromancy")
        .bind(1_i64)
        .execute(&pool)
        .await
        .unwrap();
    }

    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM subclass_spell_choice_grants")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.0, 2, "re-inserting the same two grants should dedupe to 2 rows, not 4");
}

#[tokio::test]
async fn character_crud_round_trips() {
    let pool = test_pool().await;
    insert_user(&pool, "owner-1", "owner").await;

    let mut hero = character("hero-1", "Test Hero");
    save_character(&pool, &hero).await.unwrap();
    assert_eq!(get_character(&pool, "hero-1", "owner-1").await.unwrap(), Some(hero.clone()));

    hero.name = "Renamed Hero".to_string();
    hero.classes[0].level = 6;
    save_character(&pool, &hero).await.unwrap();
    let reloaded = get_character(&pool, "hero-1", "owner-1").await.unwrap().unwrap();
    assert_eq!(reloaded.name, "Renamed Hero");
    assert_eq!(crate::models::character::total_level(&reloaded.classes), 6);

    delete_character(&pool, "hero-1", "owner-1").await.unwrap();
    assert_eq!(get_character(&pool, "hero-1", "owner-1").await.unwrap(), None);

    assert!(save_character(&pool, &character("", "No Id")).await.is_err());
}

#[tokio::test]
async fn character_access_is_scoped_to_its_owner() {
    let pool = test_pool().await;
    insert_user(&pool, "owner-1", "owner-one").await;
    insert_user(&pool, "owner-2", "owner-two").await;

    save_character(&pool, &character("hero-1", "Test Hero")).await.unwrap();

    // A different owner can't fetch, list, edit, or delete it.
    assert_eq!(get_character(&pool, "hero-1", "owner-2").await.unwrap(), None);
    assert_eq!(list_characters(&pool, "owner-2").await.unwrap(), vec![]);
    assert_eq!(list_characters(&pool, "owner-1").await.unwrap().len(), 1);

    delete_character(&pool, "hero-1", "owner-2").await.unwrap();
    assert!(get_character(&pool, "hero-1", "owner-1").await.unwrap().is_some());
}

#[tokio::test]
async fn list_characters_resolves_display_names() {
    let pool = test_pool().await;
    insert_user(&pool, "owner-1", "owner").await;
    insert_class(&pool, "fake-warrior", "Fake Warrior").await;
    let s = species("Elf", "PHB", None);
    insert_flat(&pool, "species", &s.id, &s.name, &s.source, serde_json::to_string(&s).unwrap()).await;

    save_character(&pool, &character("hero-1", "Beta")).await.unwrap();
    let mut dangling = character("hero-2", "Alpha");
    dangling.species_id = "purged-species".to_string();
    save_character(&pool, &dangling).await.unwrap();

    let summaries = list_characters(&pool, "owner-1").await.unwrap();
    assert_eq!(summaries.len(), 2);
    assert_eq!(summaries[0].name, "Alpha");
    assert_eq!(summaries[0].species_name, None);
    assert_eq!(summaries[1].name, "Beta");
    assert_eq!(summaries[1].class_name.as_deref(), Some("Fake Warrior"));
    assert_eq!(summaries[1].species_name.as_deref(), Some("Elf"));
}

#[tokio::test]
async fn character_sheet_resolves_refs_and_tolerates_dangling_ones() {
    let pool = test_pool().await;
    insert_user(&pool, "owner-1", "owner").await;
    insert_class(&pool, "fake-warrior", "Fake Warrior").await;
    let s = species("Elf", "PHB", None);
    insert_flat(&pool, "species", &s.id, &s.name, &s.source, serde_json::to_string(&s).unwrap()).await;
    let b = background("Acolyte", "PHB");
    insert_flat(&pool, "backgrounds", &b.id, &b.name, &b.source, serde_json::to_string(&b).unwrap()).await;
    let f = feat("Grappler", "PHB", None);
    insert_flat(&pool, "feats", &f.id, &f.name, &f.source, serde_json::to_string(&f).unwrap()).await;

    save_character(&pool, &character("hero-1", "Test Hero")).await.unwrap();
    let sheet = get_character_sheet(&pool, "hero-1", "owner-1").await.unwrap().unwrap();
    assert_eq!(sheet.species.unwrap().name, "Elf");
    assert_eq!(sheet.background.unwrap().name, "Acolyte");
    assert_eq!(sheet.classes[0].class.name, "Fake Warrior");
    assert_eq!(sheet.feats.len(), 1);
    assert_eq!(sheet.feats[0].name, "Grappler");

    let mut dangling = character("hero-2", "Ghost");
    dangling.species_id = "purged".to_string();
    dangling.classes[0].class_id = "purged".to_string();
    dangling.asi_choices = vec![Some(AsiChoice::Feat { feat_id: "purged".to_string() })];
    save_character(&pool, &dangling).await.unwrap();
    let sheet = get_character_sheet(&pool, "hero-2", "owner-1").await.unwrap().unwrap();
    assert_eq!(sheet.species, None);
    assert!(sheet.classes.is_empty());
    assert!(sheet.feats.is_empty());

    assert!(get_character_sheet(&pool, "nope", "owner-1").await.unwrap().is_none());
}

#[tokio::test]
async fn character_sheet_resolves_granted_spells_for_active_subclass() {
    let pool = test_pool().await;
    insert_user(&pool, "owner-1", "owner").await;
    insert_class(&pool, "fake-warrior", "Fake Warrior").await;
    insert_subclass(&pool, "fake-life-domain", "fake-warrior", "Fake Life Domain").await;
    // Gives the subclass an unlock level (1) so `active_subclass` treats it
    // as active for this level-5 character.
    let feature = crate::models::class::ClassFeature {
        name: "Domain Spells".to_string(),
        source: "TBK".to_string(),
        level: 1,
        entries: vec![],
    };
    sqlx::query(
        "INSERT INTO subclass_features (subclass_id, name, source, level, sort_order, data_json) VALUES (?,?,?,?,?,?)",
    )
    .bind("fake-life-domain")
    .bind(&feature.name)
    .bind(&feature.source)
    .bind(feature.level)
    .bind(0_i64)
    .bind(serde_json::to_string(&feature).unwrap())
    .execute(&pool)
    .await
    .unwrap();

    spell_fixtures(&pool).await; // Fireball(3), Fire Bolt(0), Charm Person(1)
    sqlx::query("INSERT INTO subclass_granted_spells (subclass_id, spell_id, grant_level) VALUES (?, ?, ?)")
        .bind("fake-life-domain")
        .bind("charm-person")
        .bind(1_i64)
        .execute(&pool)
        .await
        .unwrap();

    let mut hero = character("hero-1", "Test Hero");
    hero.classes[0].subclass_id = Some("fake-life-domain".to_string());
    save_character(&pool, &hero).await.unwrap();

    let sheet = get_character_sheet(&pool, "hero-1", "owner-1").await.unwrap().unwrap();
    assert_eq!(
        sheet.granted_spells.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(),
        vec!["Charm Person"]
    );
}

#[tokio::test]
async fn bootstrap_admin_if_empty_creates_one_owner_and_is_idempotent() {
    let pool = test_pool().await;

    bootstrap_admin_if_empty(&pool).await.unwrap();
    let users = list_users(&pool).await.unwrap();
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].username, "admin");
    assert!(users[0].is_instance_owner);

    let admin_id = authenticate(&pool, "admin", "admin").await.unwrap().unwrap();
    assert_eq!(admin_id, users[0].id);

    // Calling it again (e.g. on a subsequent server boot) must not duplicate
    // the account or touch any existing users.
    bootstrap_admin_if_empty(&pool).await.unwrap();
    assert_eq!(list_users(&pool).await.unwrap().len(), 1);
}

#[tokio::test]
async fn authenticate_rejects_wrong_password_and_unknown_username() {
    let pool = test_pool().await;
    create_user(&pool, "gm", "correct-horse", false).await.unwrap();

    assert!(authenticate(&pool, "gm", "correct-horse").await.unwrap().is_some());
    assert!(authenticate(&pool, "gm", "wrong-password").await.unwrap().is_none());
    assert!(authenticate(&pool, "nobody", "correct-horse").await.unwrap().is_none());
}

#[tokio::test]
async fn create_user_rejects_duplicate_username() {
    let pool = test_pool().await;
    create_user(&pool, "gm", "password1", false).await.unwrap();

    let err = create_user(&pool, "gm", "password2", false).await.unwrap_err();
    assert!(err.to_string().contains("already taken"));
}

#[tokio::test]
async fn grant_and_revoke_permission_round_trip() {
    let pool = test_pool().await;
    let admin_id = create_user(&pool, "admin", "admin", true).await.unwrap();
    let user_id = create_user(&pool, "player", "password", false).await.unwrap();

    grant_permission(&pool, &user_id, Permission::CreateHomebrew, &admin_id).await.unwrap();
    // Granting the same permission twice must not error or duplicate the row.
    grant_permission(&pool, &user_id, Permission::CreateHomebrew, &admin_id).await.unwrap();

    let user = get_current_user(&pool, &user_id).await.unwrap().unwrap();
    assert_eq!(user.permissions, vec![Permission::CreateHomebrew]);

    revoke_permission(&pool, &user_id, Permission::CreateHomebrew).await.unwrap();
    let user = get_current_user(&pool, &user_id).await.unwrap().unwrap();
    assert!(user.permissions.is_empty());
}

#[tokio::test]
async fn current_user_is_admin_and_has_permission_logic() {
    let pool = test_pool().await;
    let user_id = create_user(&pool, "owner", "admin", true).await.unwrap();
    let manager_id = create_user(&pool, "manager", "password", false).await.unwrap();
    let plain_id = create_user(&pool, "player", "password", false).await.unwrap();

    grant_permission(&pool, &manager_id, Permission::ManageUsers, &user_id).await.unwrap();
    grant_permission(&pool, &plain_id, Permission::ImportSources, &user_id).await.unwrap();

    let owner = get_current_user(&pool, &user_id).await.unwrap().unwrap();
    let manager = get_current_user(&pool, &manager_id).await.unwrap().unwrap();
    let plain = get_current_user(&pool, &plain_id).await.unwrap().unwrap();

    assert!(owner.is_admin());
    assert!(manager.is_admin());
    assert!(!plain.is_admin());

    // Admins are treated as having every permission, even ones never
    // explicitly granted to them.
    assert!(owner.has_permission(Permission::ArchiveSources));
    assert!(manager.has_permission(Permission::ArchiveSources));
    assert!(plain.has_permission(Permission::ImportSources));
    assert!(!plain.has_permission(Permission::ArchiveSources));
}

#[tokio::test]
async fn update_username_rejects_taken_name() {
    let pool = test_pool().await;
    create_user(&pool, "taken", "password", false).await.unwrap();
    let user_id = create_user(&pool, "player", "password", false).await.unwrap();

    let err = update_username(&pool, &user_id, "taken").await.unwrap_err();
    assert!(err.to_string().contains("already taken"));

    update_username(&pool, &user_id, "renamed").await.unwrap();
    let user = get_current_user(&pool, &user_id).await.unwrap().unwrap();
    assert_eq!(user.username, "renamed");
}

#[tokio::test]
async fn new_users_default_to_the_guidance_theme_and_can_change_it() {
    let pool = test_pool().await;
    let user_id = create_user(&pool, "player", "password", false).await.unwrap();

    let user = get_current_user(&pool, &user_id).await.unwrap().unwrap();
    assert_eq!(user.theme, "guidance");

    update_theme(&pool, &user_id, "dracula").await.unwrap();
    let user = get_current_user(&pool, &user_id).await.unwrap().unwrap();
    assert_eq!(user.theme, "dracula");
}
