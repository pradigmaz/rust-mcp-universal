use std::collections::BTreeSet;
use std::path::PathBuf;

use serde_json::{Map, Value};

use crate::path_input::resolve_existing_directory_input;
use crate::state::{ProjectBinding, ProjectBindingSource, ServerState};

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
        .and_then(resolve_existing_directory_input)
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
                if let Some(project_path) = resolve_existing_directory_input(raw) {
                    candidates.insert(project_path);
                }
            }
            Value::Object(object) => {
                for key in ["path", "uri", "rootPath", "rootUri"] {
                    if let Some(project_path) = object
                        .get(key)
                        .and_then(Value::as_str)
                        .and_then(resolve_existing_directory_input)
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
    if let Some(project_path) = resolve_existing_directory_input(raw) {
        candidates.insert(project_path);
    }
}
