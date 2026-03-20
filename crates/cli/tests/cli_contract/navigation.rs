use assert_cmd::cargo::cargo_bin_cmd;
use tempfile::tempdir;

#[test]
fn symbol_lookup_with_auto_index_returns_matches() {
    let project = tempdir().expect("temp dir");
    std::fs::create_dir_all(project.path().join("src")).expect("create src");
    std::fs::write(
        project.path().join("src/main.rs"),
        "fn lookup_symbol_target() {}\n",
    )
    .expect("write fixture");
    let project_path = project.path().to_str().expect("utf-8 path");

    let assert = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            project_path,
            "--json",
            "symbol-lookup",
            "--name",
            "lookup_symbol_target",
            "--auto-index",
        ])
        .assert()
        .success();

    let stdout =
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout should be utf-8");
    let payload: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should contain JSON");
    assert!(payload.as_array().is_some_and(|items| !items.is_empty()));
    assert!(payload[0]["path"].is_string());
    assert!(payload[0]["line"].is_number());
    assert!(payload[0]["column"].is_number());
}

#[test]
fn symbol_references_returns_grouped_hits() {
    let project = tempdir().expect("temp dir");
    std::fs::create_dir_all(project.path().join("src")).expect("create src");
    std::fs::write(
        project.path().join("src/lib.rs"),
        r#"
fn reference_target() {}
mod caller {
    fn call() {
        crate::reference_target();
    }
}
"#,
    )
    .expect("write fixture");
    let project_path = project.path().to_str().expect("utf-8 path");

    let assert = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            project_path,
            "--json",
            "symbol-references",
            "--name",
            "reference_target",
            "--auto-index",
        ])
        .assert()
        .success();

    let stdout =
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout should be utf-8");
    let payload: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should contain JSON");
    assert!(payload.as_array().is_some_and(|items| !items.is_empty()));
    assert!(payload[0]["ref_count"].is_number());
    assert!(payload[0]["line"].is_number());
    assert!(payload[0]["column"].is_number());
}

#[test]
fn symbol_references_returns_type_and_struct_literal_usages() {
    let project = tempdir().expect("temp dir");
    std::fs::create_dir_all(project.path().join("src")).expect("create src");
    std::fs::write(
        project.path().join("src/lib.rs"),
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
    )
    .expect("write fixture");
    let project_path = project.path().to_str().expect("utf-8 path");

    let assert = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            project_path,
            "--json",
            "symbol-references",
            "--name",
            "GraphRef",
            "--auto-index",
        ])
        .assert()
        .success();

    let stdout =
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout should be utf-8");
    let payload: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should contain JSON");
    assert!(
        payload
            .as_array()
            .is_some_and(|items| items.iter().any(|item| {
                item["path"] == "src/lib.rs"
                    && item["exact"] == serde_json::json!(true)
                    && item["ref_count"].as_u64().is_some_and(|count| count >= 5)
                    && item["line"].is_number()
                    && item["column"].is_number()
            }))
    );
}

#[test]
fn related_files_returns_connected_neighbors() {
    let project = tempdir().expect("temp dir");
    std::fs::create_dir_all(project.path().join("src")).expect("create src");
    std::fs::write(
        project.path().join("src/main.rs"),
        r#"
use crate::shared::helper;
fn root() {
    helper();
}
"#,
    )
    .expect("write main");
    std::fs::write(
        project.path().join("src/shared.rs"),
        r#"
fn helper() {}
"#,
    )
    .expect("write shared");
    let project_path = project.path().to_str().expect("utf-8 path");

    let assert = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            project_path,
            "--json",
            "related-files",
            "--path",
            "src/main.rs",
            "--auto-index",
        ])
        .assert()
        .success();

    let stdout =
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout should be utf-8");
    let payload: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should contain JSON");
    assert!(
        payload
            .as_array()
            .is_some_and(|items| items.iter().any(|item| item["path"] == "src/shared.rs"))
    );
}

#[test]
fn call_path_returns_path_with_evidence() {
    let project = tempdir().expect("temp dir");
    std::fs::create_dir_all(project.path().join("src")).expect("create src");
    std::fs::write(
        project.path().join("src/main.rs"),
        "mod shared;\nmod util;\nfn main() { shared::helper(); }\n",
    )
    .expect("write main");
    std::fs::write(
        project.path().join("src/shared.rs"),
        "pub fn helper() { crate::util::support(); }\n",
    )
    .expect("write shared");
    std::fs::write(project.path().join("src/util.rs"), "pub fn support() {}\n")
        .expect("write util");
    let project_path = project.path().to_str().expect("utf-8 path");

    let assert = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            project_path,
            "--json",
            "call-path",
            "--from",
            "src/main.rs",
            "--to",
            "support",
            "--auto-index",
        ])
        .assert()
        .success();

    let stdout =
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout should be utf-8");
    let payload: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should contain JSON");
    assert_eq!(payload["found"], serde_json::json!(true));
    assert_eq!(payload["hops"], serde_json::json!(2));
    assert_eq!(payload["path"][0], "src/main.rs");
    assert_eq!(payload["path"][2], "src/util.rs");
    assert_eq!(payload["steps"][0]["edge_kind"], "ref_tail_unique");
    assert!(
        payload["steps"][0]["evidence"]
            .as_str()
            .is_some_and(|value| value.contains("helper"))
    );
    assert!(
        payload["steps"][1]["evidence"]
            .as_str()
            .is_some_and(|value| value.contains("support"))
    );
}

#[test]
fn context_pack_design_mode_returns_docs_first() {
    let project = tempdir().expect("temp dir");
    std::fs::create_dir_all(project.path().join("src")).expect("create src");
    std::fs::create_dir_all(project.path().join("docs")).expect("create docs");
    std::fs::write(
        project.path().join("src/lib.rs"),
        "pub fn architecture_runtime() {}\n",
    )
    .expect("write source");
    std::fs::write(
        project.path().join("docs/design.md"),
        "Architecture overview and design decisions.\n",
    )
    .expect("write docs");
    let project_path = project.path().to_str().expect("utf-8 path");

    let assert = cargo_bin_cmd!("rmu-cli")
        .args([
            "--project-path",
            project_path,
            "--json",
            "context-pack",
            "--query",
            "architecture",
            "--mode",
            "design",
            "--auto-index",
        ])
        .assert()
        .success();

    let stdout =
        String::from_utf8(assert.get_output().stdout.clone()).expect("stdout should be utf-8");
    let payload: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should contain JSON");
    assert_eq!(payload["mode"], serde_json::json!("design"));
    assert_eq!(payload["context"]["files"][0]["path"], "docs/design.md");
}
