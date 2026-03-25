pub fn reconcile_member(member_id: &str) -> bool {
    validate_member(member_id);
    let _query = sqlx::query!(
        "SELECT id FROM members WHERE member_key = $1",
        member_id
    );
    true
}
