CREATE TABLE subclass_spell_choice_grants (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    subclass_id TEXT NOT NULL REFERENCES subclasses(id),
    grant_level INTEGER NOT NULL,
    spell_level INTEGER NOT NULL,
    class_name TEXT,
    school TEXT,
    count INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX idx_subclass_spell_choice_grants_subclass ON subclass_spell_choice_grants(subclass_id);

-- COALESCE'd because SQLite's UNIQUE treats every NULL as distinct, which
-- would otherwise let a re-seed duplicate rows where class_name/school is NULL.
CREATE UNIQUE INDEX idx_subclass_spell_choice_grants_unique
    ON subclass_spell_choice_grants(
        subclass_id,
        grant_level,
        spell_level,
        COALESCE(class_name, ''),
        COALESCE(school, '')
    );
