use assert_cmd::cargo::cargo_bin_cmd;
use tempfile::tempdir;

#[test]
fn symbol_lookup_auto_index_finds_decorated_python_async_function() {
    let project = tempdir().expect("temp dir");
    std::fs::create_dir_all(project.path().join("src")).expect("create src");
    std::fs::write(
        project.path().join("src/advanced.py"),
        r#"def traced(fn):
    return fn


@traced
async def decorated_worker():
    return 1
"#,
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
            "decorated_worker",
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
            .is_some_and(|items| items.iter().any(|hit| hit["path"] == "src/advanced.py"))
    );
}
