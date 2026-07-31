-- Weapon property codes are the one genuinely multi-valued item filter
-- dimension (a Greatsword is both Heavy and Two-Handed) — see
-- optional_feature_prerequisite_* for the same join-table-per-filter
-- pattern. `property_label` is denormalized onto the join row so the
-- filter's option list needs no separate lookup at query time.
CREATE TABLE item_properties (
    item_id        TEXT NOT NULL REFERENCES items(id),
    property_code  TEXT NOT NULL,
    property_label TEXT NOT NULL,
    PRIMARY KEY (item_id, property_code)
);

CREATE INDEX idx_item_properties_code ON item_properties(property_code);
