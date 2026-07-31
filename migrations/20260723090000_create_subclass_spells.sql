CREATE TABLE subclass_spells (
    subclass_id TEXT NOT NULL REFERENCES subclasses(id),
    spell_id TEXT NOT NULL REFERENCES spells(id),
    PRIMARY KEY (subclass_id, spell_id)
);

CREATE INDEX idx_subclass_spells_subclass ON subclass_spells(subclass_id);
