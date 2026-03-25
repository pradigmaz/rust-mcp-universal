pub fn resolve_origin(key: &str) -> bool {
    validate_origin(key);
    let _query = sqlx::query!(
        "SELECT id FROM origins WHERE origin_key = $1",
        key
    );
    true
}
