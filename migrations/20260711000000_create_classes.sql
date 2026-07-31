CREATE TABLE classes (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    source TEXT NOT NULL,
    hit_die INTEGER NOT NULL,
    data_json TEXT NOT NULL
);

CREATE TABLE subclasses (
    id TEXT PRIMARY KEY,
    class_id TEXT NOT NULL REFERENCES classes(id),
    name TEXT NOT NULL,
    short_name TEXT NOT NULL,
    source TEXT NOT NULL,
    data_json TEXT NOT NULL
);

CREATE TABLE class_features (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    class_id TEXT NOT NULL REFERENCES classes(id),
    name TEXT NOT NULL,
    source TEXT NOT NULL,
    level INTEGER NOT NULL,
    sort_order INTEGER NOT NULL,
    data_json TEXT NOT NULL
);

CREATE TABLE subclass_features (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    subclass_id TEXT NOT NULL REFERENCES subclasses(id),
    name TEXT NOT NULL,
    source TEXT NOT NULL,
    level INTEGER NOT NULL,
    sort_order INTEGER NOT NULL,
    data_json TEXT NOT NULL
);

CREATE INDEX idx_subclasses_class ON subclasses(class_id);
CREATE INDEX idx_class_features_class ON class_features(class_id, level, sort_order);
CREATE INDEX idx_subclass_features_subclass ON subclass_features(subclass_id, level, sort_order);
