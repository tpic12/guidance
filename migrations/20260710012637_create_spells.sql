CREATE TABLE spells (
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL,
    level         INTEGER NOT NULL,
    school        TEXT NOT NULL,
    source        TEXT NOT NULL,
    concentration BOOLEAN NOT NULL,
    ritual        BOOLEAN NOT NULL,
    data_json     TEXT NOT NULL CHECK(json_valid(data_json))
);