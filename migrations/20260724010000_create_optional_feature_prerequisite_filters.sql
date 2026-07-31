-- Denormalized, filterable views of `optional_features.data_json`'s
-- `prerequisites` array, mirroring `optional_feature_types`: a feature can
-- have more than one prerequisite requirement worth filtering by (e.g. two
-- OR-alternative pacts), so these can't be single columns on
-- `optional_features` itself.

CREATE TABLE optional_feature_prerequisite_pacts (
    optional_feature_id TEXT NOT NULL REFERENCES optional_features(id),
    pact TEXT NOT NULL,
    PRIMARY KEY (optional_feature_id, pact)
);

CREATE INDEX idx_optional_feature_prerequisite_pacts_pact
    ON optional_feature_prerequisite_pacts(pact);

-- `class_requirement` is the combined display label ("Warlock", or
-- "Fighter (Rune Knight)" when the option also names a subclass) — kept as
-- one filterable value per `PrerequisiteOption::class_requirement_label`,
-- since a bare class alongside a class+subclass combo aren't really the
-- same filter dimension a user would cross-select.
CREATE TABLE optional_feature_prerequisite_classes (
    optional_feature_id TEXT NOT NULL REFERENCES optional_features(id),
    class_requirement TEXT NOT NULL,
    PRIMARY KEY (optional_feature_id, class_requirement)
);

CREATE INDEX idx_optional_feature_prerequisite_classes_req
    ON optional_feature_prerequisite_classes(class_requirement);
