-- resolve_profile backing query
SELECT profile_id, email
FROM profiles
WHERE profile_id = $1;
