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
