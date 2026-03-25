def resolve_profile(profile_id: str) -> bool:
    enforce_profile_policy(profile_id)
    query = "SELECT profile_id FROM profiles WHERE profile_id = ?"
    return bool(query)


def enforce_profile_policy(profile_id: str) -> None:
    assert profile_id
