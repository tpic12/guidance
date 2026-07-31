use crate::importer::parse_class_spells::RawSpellClassLookup;

/// One resolvable class<->spell grant, prior to matching against the
/// already-seeded `classes`/`spells` tables (by name; `grantSource` from the
/// raw lookup isn't needed for that match, so it's dropped here).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassSpellLink {
    pub spell_name_lower: String,
    pub spell_source: String,
    pub class_name: String,
}

/// One resolvable subclass<->spell grant. Subclass identity needs
/// `class_name` + `subclass_short_name` + `subclass_source` together to
/// match `Subclass::id`'s `slugify("{class} {short_name} {source}")` scheme
/// (see `transform_class::class_bundles_from_parsed`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubclassSpellLink {
    pub spell_name_lower: String,
    pub spell_source: String,
    pub class_name: String,
    pub subclass_short_name: String,
    pub subclass_source: String,
}

pub fn class_spell_links_from_parsed(lookup: &RawSpellClassLookup) -> Vec<ClassSpellLink> {
    let mut links = Vec::new();
    for (spell_source, spells) in lookup {
        for (spell_name_lower, entry) in spells {
            for classes in entry.class.values() {
                for class_name in classes.keys() {
                    links.push(ClassSpellLink {
                        spell_name_lower: spell_name_lower.clone(),
                        spell_source: spell_source.clone(),
                        class_name: class_name.clone(),
                    });
                }
            }
        }
    }
    links
}

pub fn subclass_spell_links_from_parsed(lookup: &RawSpellClassLookup) -> Vec<SubclassSpellLink> {
    let mut links = Vec::new();
    for (spell_source, spells) in lookup {
        for (spell_name_lower, entry) in spells {
            // subclassSource -> className -> classSource -> subclassShortName -> value.
            for (subclass_source, by_class) in &entry.subclass {
                for (class_name, by_class_source) in by_class {
                    for by_short_name in by_class_source.values() {
                        for subclass_short_name in by_short_name.keys() {
                            links.push(SubclassSpellLink {
                                spell_name_lower: spell_name_lower.clone(),
                                spell_source: spell_source.clone(),
                                class_name: class_name.clone(),
                                subclass_short_name: subclass_short_name.clone(),
                                subclass_source: subclass_source.clone(),
                            });
                        }
                    }
                }
            }
        }
    }
    links
}

#[cfg(test)]
#[path = "transform_class_spells_tests.rs"]
mod tests;
