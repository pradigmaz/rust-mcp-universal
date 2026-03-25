use crate::policies::profile_policy::enforce_profile_policy;

pub fn resolve_profile(profile_id: &str) -> bool {
    enforce_profile_policy(profile_id);
    let _query = include_str!("../repositories/profile_repo.sql");
    !profile_id.is_empty()
}
