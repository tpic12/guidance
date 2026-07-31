CREATE TABLE characters (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    user_id    TEXT NOT NULL REFERENCES users(id),
    data_json  TEXT NOT NULL CHECK(json_valid(data_json)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
