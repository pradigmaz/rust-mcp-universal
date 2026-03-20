use super::*;

#[test]
fn symbol_lookup_returns_exact_match_before_partial_matches() -> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-symbol-lookup");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(
        project_dir.join("src/main.rs"),
        "pub async fn alpha_beta_gamma() {}\npub(crate) fn alpha_beta() {}\n",
    )?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    engine.index_path()?;

    let matches = engine.symbol_lookup("alpha_beta_gamma", 10)?;
    assert!(!matches.is_empty());
    assert_eq!(matches[0].name, "alpha_beta_gamma");
    assert!(matches[0].exact);
    assert_eq!(matches[0].line, Some(1));
    assert_eq!(matches[0].column, Some(14));

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn symbol_references_groups_reference_hits_by_file() -> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-symbol-references");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(
        project_dir.join("src/lib.rs"),
        "pub fn shared_symbol() {}\nmod other {\n    pub(crate) fn call() {\n        crate::shared_symbol();\n        shared_symbol();\n    }\n}\n",
    )?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    engine.index_path()?;

    let hits = engine.symbol_references("shared_symbol", 10)?;
    assert!(hits.iter().any(|hit| {
        (hit.path == "src/lib.rs" || hit.path.ends_with("src/lib.rs")) && hit.ref_count >= 2
    }));
    assert!(
        hits.iter()
            .any(|hit| { hit.exact && hit.line == Some(5) && hit.column == Some(9) })
    );

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn symbol_references_surface_type_and_struct_literal_usages() -> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-symbol-references-types");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(
        project_dir.join("src/lib.rs"),
        r#"
pub struct GraphRef {
    value: usize,
}

pub struct Holder {
    inner: GraphRef,
}

impl GraphRef {
    pub fn from_value(value: usize) -> Self {
        GraphRef { value }
    }
}

fn mirror(input: &GraphRef) -> GraphRef {
    GraphRef { value: input.value }
}
"#,
    )?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    engine.index_path()?;

    let hits = engine.symbol_references("GraphRef", 10)?;
    assert!(hits.iter().any(|hit| {
        (hit.path == "src/lib.rs" || hit.path.ends_with("src/lib.rs"))
            && hit.exact
            && hit.ref_count >= 5
            && hit.line.is_some()
            && hit.column.is_some()
    }));

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn related_files_returns_files_connected_by_calls_and_shared_deps() -> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-related-files");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(
        project_dir.join("src/main.rs"),
        "pub(crate) use crate::shared::helper;\n\npub async fn entry() {\n    helper();\n}\n",
    )?;
    fs::write(
        project_dir.join("src/shared.rs"),
        "pub use crate::util::support;\n\npub(super) fn helper() {\n    support();\n}\n",
    )?;
    fs::write(project_dir.join("src/util.rs"), "pub fn support() {}\n")?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    engine.index_path()?;

    let related = engine.related_files("src/main.rs", 10)?;
    assert!(related.iter().any(|hit| hit.path == "src/shared.rs"));
    assert!(related.iter().all(|hit| hit.path != "src/main.rs"));

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn call_path_returns_bounded_path_with_evidence() -> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-call-path");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(
        project_dir.join("src/main.rs"),
        "mod shared;\nmod util;\nfn main() { shared::helper(); }\n",
    )?;
    fs::write(
        project_dir.join("src/shared.rs"),
        "pub fn helper() { crate::util::support(); }\n",
    )?;
    fs::write(project_dir.join("src/util.rs"), "pub fn support() {}\n")?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    engine.index_path()?;

    let result = engine.call_path("src/main.rs", "support", 6)?;
    assert!(result.found);
    assert_eq!(result.hops, 2);
    assert_eq!(
        result.path,
        vec!["src/main.rs", "src/shared.rs", "src/util.rs"]
    );
    assert_eq!(result.steps.len(), 2);
    assert_eq!(result.steps[0].edge_kind, "ref_tail_unique");
    assert_eq!(result.steps[0].to_path, "src/shared.rs");
    assert!(result.steps[0].evidence.contains("helper"));
    assert_eq!(result.steps[1].to_path, "src/util.rs");
    assert!(result.steps[1].evidence.contains("support"));
    assert!(
        result
            .steps
            .iter()
            .all(|step| step.line.is_some() && step.column.is_some())
    );

    cleanup_project(&project_dir);
    Ok(())
}
