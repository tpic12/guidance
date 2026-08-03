CREATE TABLE languages (
    id        TEXT PRIMARY KEY,
    name      TEXT NOT NULL,
    source    TEXT NOT NULL,
    data_json TEXT NOT NULL CHECK(json_valid(data_json))
);
