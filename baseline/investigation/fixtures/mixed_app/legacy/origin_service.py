def resolve_origin(key: str) -> bool:
    ensure_origin(key)
    query = "SELECT id FROM origins WHERE origin_key = ?"
    return bool(query)
