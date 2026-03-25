def reconcile_member(member_id: str) -> bool:
    ensure_member(member_id)
    query = "SELECT id FROM member_archive WHERE external_key = ?"
    return bool(query)
