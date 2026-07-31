use super::*;

#[test]
fn blocks_mock_fixtures_into_the_default_dev_db() {
    assert!(is_unsafe_mock_seed(MOCK_FIXTURES_DIR, DEFAULT_DEV_DB_URL));
}

#[test]
fn allows_mock_fixtures_into_a_dedicated_e2e_db() {
    assert!(!is_unsafe_mock_seed(MOCK_FIXTURES_DIR, "sqlite://e2e.db"));
}

#[test]
fn allows_real_fixtures_into_the_default_dev_db() {
    assert!(!is_unsafe_mock_seed("fixtures", DEFAULT_DEV_DB_URL));
}
