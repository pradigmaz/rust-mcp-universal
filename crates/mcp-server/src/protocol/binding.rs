use std::collections::BTreeSet;
use std::path::PathBuf;

use serde_json::{Map, Value};

use crate::state::{
    ProjectBinding, ProjectBindingSource, ServerState, normalize_existing_directory,
};

pub(super) fn apply_initialize_binding(params: Option<&Value>, state: &mut ServerState) {
    if matches!(
        state.binding(),
        ProjectBinding::Bound {
            source: ProjectBindingSource::Cli,
            ..
        }
    ) {
        return;
    }

    let Some(object) = params.and_then(Value::as_object) else {
        return;
    };

    match select_root_binding(object) {
        RootSelection::Bound(project_path) => {
            state.bind_project_path(project_path, ProjectBindingSource::InitializeRoots);
            return;
        }
        RootSelection::Ambiguous(candidates) => {
            state.apply_binding(ProjectBinding::Ambiguous {
                candidates,
                source: ProjectBindingSource::InitializeRoots,
            });
            return;
        }
        RootSelection::None => {}
    }

    if let Some(project_path) = object
        .get("projectPath")
        .and_then(Value::as_str)
        .and_then(resolve_existing_directory)
    {
        state.bind_project_path(project_path, ProjectBindingSource::InitializeProjectPath);
    }
}

enum RootSelection {
    None,
    Bound(PathBuf),
    Ambiguous(Vec<PathBuf>),
}

fn select_root_binding(object: &Map<String, Value>) -> RootSelection {
    let candidates = collect_root_candidates(object);
    match candidates.as_slice() {
        [] => RootSelection::None,
        [project_path] => RootSelection::Bound(project_path.clone()),
        _ => RootSelection::Ambiguous(candidates),
    }
}

fn collect_root_candidates(object: &Map<String, Value>) -> Vec<PathBuf> {
    let mut candidates = BTreeSet::new();
    extend_with_root_field(&mut candidates, object.get("rootPath"));
    extend_with_root_field(&mut candidates, object.get("rootUri"));
    extend_with_root_collection(&mut candidates, object.get("roots"));
    extend_with_root_collection(&mut candidates, object.get("workspaceFolders"));
    candidates.into_iter().collect()
}

fn extend_with_root_collection(candidates: &mut BTreeSet<PathBuf>, value: Option<&Value>) {
    let Some(items) = value.and_then(Value::as_array) else {
        return;
    };
    for item in items {
        match item {
            Value::String(raw) => {
                if let Some(project_path) = resolve_existing_directory(raw) {
                    candidates.insert(project_path);
                }
            }
            Value::Object(object) => {
                for key in ["path", "uri", "rootPath", "rootUri"] {
                    if let Some(project_path) = object
                        .get(key)
                        .and_then(Value::as_str)
                        .and_then(resolve_existing_directory)
                    {
                        candidates.insert(project_path);
                    }
                }
            }
            _ => {}
        }
    }
}

fn extend_with_root_field(candidates: &mut BTreeSet<PathBuf>, value: Option<&Value>) {
    let Some(raw) = value.and_then(Value::as_str) else {
        return;
    };
    if let Some(project_path) = resolve_existing_directory(raw) {
        candidates.insert(project_path);
    }
}

fn resolve_existing_directory(raw: &str) -> Option<PathBuf> {
    let path = parse_path_like(raw)?;
    normalize_existing_directory(&path)
}

fn parse_path_like(raw: &str) -> Option<PathBuf> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(path) = parse_file_uri(trimmed) {
        return Some(path);
    }
    Some(PathBuf::from(trimmed))
}

fn parse_file_uri(raw: &str) -> Option<PathBuf> {
    let remainder = raw.strip_prefix("file://")?;
    let decoded = percent_decode(remainder);
    let normalized = if cfg!(windows) {
        let without_drive_prefix =
            if decoded.starts_with('/') && decoded.as_bytes().get(2) == Some(&b':') {
                decoded[1..].to_string()
            } else {
                decoded
            };
        without_drive_prefix.replace('/', "\\")
    } else {
        decoded
    };
    Some(PathBuf::from(normalized))
}

fn percent_decode(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut index = 0usize;
    let mut decoded = Vec::with_capacity(raw.len());
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let pair = &raw[index + 1..index + 3];
            if let Ok(value) = u8::from_str_radix(pair, 16) {
                decoded.push(value);
                index += 3;
                continue;
            }
        }
        decoded.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}
