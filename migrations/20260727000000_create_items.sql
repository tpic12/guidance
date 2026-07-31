CREATE TABLE items (
    id                  TEXT PRIMARY KEY,
    name                TEXT NOT NULL,
    source              TEXT NOT NULL,
    is_group            INTEGER NOT NULL DEFAULT 0,
    item_type           TEXT,
    rarity              TEXT NOT NULL DEFAULT 'none',
    rarity_rank         INTEGER NOT NULL DEFAULT 0,
    requires_attunement INTEGER NOT NULL DEFAULT 0,
    weapon_category     TEXT,
    damage_type         TEXT,
    value_cp            REAL,
    weight_lb           REAL,
    data_json           TEXT NOT NULL CHECK(json_valid(data_json))
);

CREATE INDEX idx_items_type ON items(item_type);
CREATE INDEX idx_items_rarity ON items(rarity);
CREATE INDEX idx_items_requires_attunement ON items(requires_attunement);
CREATE INDEX idx_items_weapon_category ON items(weapon_category);
CREATE INDEX idx_items_damage_type ON items(damage_type);
CREATE INDEX idx_items_is_group ON items(is_group);
