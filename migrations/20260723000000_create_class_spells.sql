CREATE TABLE class_spells (
    class_id TEXT NOT NULL REFERENCES classes(id),
    spell_id TEXT NOT NULL REFERENCES spells(id),
    PRIMARY KEY (class_id, spell_id)
);

CREATE INDEX idx_class_spells_class ON class_spells(class_id);
