CREATE TABLE subclass_granted_spells (
    subclass_id TEXT NOT NULL REFERENCES subclasses(id),
    spell_id TEXT NOT NULL REFERENCES spells(id),
    grant_level INTEGER NOT NULL,
    PRIMARY KEY (subclass_id, spell_id, grant_level)
);

CREATE INDEX idx_subclass_granted_spells_subclass ON subclass_granted_spells(subclass_id);
