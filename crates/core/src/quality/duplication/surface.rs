use crate::quality::duplication::artifact::DuplicationSignalRole;

pub(crate) fn signal_token_floor_for_surface(
    path: &str,
    language: &str,
    role: DuplicationSignalRole,
) -> i64 {
    let lowered = normalize_path(path);
    let mut floor = if lowered.ends_with(".tsx") || lowered.ends_with(".jsx") {
        220
    } else if lowered.ends_with(".py") {
        128
    } else if matches!(language, "typescript" | "javascript" | "java" | "rust") {
        96
    } else {
        64
    };
    if is_modelish_path(&lowered) || role == DuplicationSignalRole::Downweighted {
        floor = floor.max(192);
    }
    if is_wrapper_surface_path(&lowered) {
        floor = floor.max(256);
    }
    floor
}

pub(crate) fn paths_all(member_paths: &[String], predicate: fn(&str) -> bool) -> bool {
    !member_paths.is_empty()
        && member_paths
            .iter()
            .map(|path| normalize_path(path))
            .all(|path| predicate(&path))
}

pub(crate) fn normalize_path(path: &str) -> String {
    path.replace('\\', "/").to_ascii_lowercase()
}

pub(crate) fn is_modelish_path(lowered: &str) -> bool {
    lowered.contains("/model")
        || lowered.contains("/models/")
        || lowered.contains("/schema")
        || lowered.contains("/schemas/")
        || lowered.contains("/dto")
        || lowered.contains("/entity")
        || lowered.contains("/entities/")
        || lowered.contains("/serializer")
        || lowered.contains("/serializers/")
        || lowered.contains("/payload")
        || lowered.contains("/request")
        || lowered.contains("/response")
        || lowered.contains("/record")
        || lowered.contains("/records/")
        || lowered.contains("/config/")
        || lowered.contains("/settings/")
}

pub(crate) fn is_wrapper_surface_path(lowered: &str) -> bool {
    lowered.contains("/components/")
        || lowered.contains("/layout")
        || lowered.contains("/layouts/")
        || lowered.contains("/page")
        || lowered.contains("/pages/")
        || lowered.contains("/dialog")
        || lowered.contains("/modal")
        || lowered.contains("/sheet")
        || lowered.contains("/panel")
        || lowered.contains("/card")
        || lowered.contains("/provider")
        || lowered.contains("/view")
}
