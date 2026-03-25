use std::collections::BTreeSet;

use serde_json::Value;

pub(super) fn found_expected_entry_path<'a, I>(variants: I, expected_paths: &[String]) -> bool
where
    I: IntoIterator<Item = &'a Value>,
{
    variants.into_iter().any(|variant| {
        variant["entry_anchor"]["path"]
            .as_str()
            .is_some_and(|path| expected_paths.iter().any(|expected| expected == path))
    })
}

pub(super) fn found_expected_route_entry<'a, I>(routes: I, expected_paths: &[String]) -> bool
where
    I: IntoIterator<Item = &'a Value>,
{
    routes.into_iter().any(|route| {
        route["segments"]
            .as_array()
            .and_then(|segments| segments.first())
            .and_then(|segment| segment["path"].as_str())
            .is_some_and(|path| expected_paths.iter().any(|expected| expected == path))
    })
}

pub(super) fn count_expected_entry_paths<'a, I>(variants: I, expected_paths: &[String]) -> usize
where
    I: IntoIterator<Item = &'a Value>,
{
    variants
        .into_iter()
        .filter_map(|variant| variant["entry_anchor"]["path"].as_str())
        .filter(|path| expected_paths.iter().any(|expected| expected == *path))
        .collect::<BTreeSet<_>>()
        .len()
}

pub(super) fn count_expected_route_entry_paths<'a, I>(routes: I, expected_paths: &[String]) -> usize
where
    I: IntoIterator<Item = &'a Value>,
{
    routes
        .into_iter()
        .filter_map(|route| {
            route["segments"]
                .as_array()
                .and_then(|segments| segments.first())
        })
        .filter_map(|segment| segment["path"].as_str())
        .filter(|path| expected_paths.iter().any(|expected| expected == *path))
        .collect::<BTreeSet<_>>()
        .len()
}

pub(super) fn route_trace_paths(payload: &Value) -> Vec<&Value> {
    let mut routes = Vec::new();
    if payload.get("best_route").is_some() {
        routes.push(&payload["best_route"]);
    }
    routes.extend(payload["alternate_routes"].as_array().into_iter().flatten());
    routes
}
