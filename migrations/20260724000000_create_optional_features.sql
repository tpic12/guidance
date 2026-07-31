CREATE TABLE optional_features (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    source TEXT NOT NULL,
    data_json TEXT NOT NULL CHECK(json_valid(data_json))
);

CREATE TABLE optional_feature_types (
    optional_feature_id TEXT NOT NULL REFERENCES optional_features(id),
    feature_type TEXT NOT NULL,
    PRIMARY KEY (optional_feature_id, feature_type)
);

CREATE INDEX idx_optional_feature_types_type ON optional_feature_types(feature_type);
