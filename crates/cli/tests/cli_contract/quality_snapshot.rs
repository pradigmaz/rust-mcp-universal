use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;
use tempfile::tempdir;

fn write_hot_file(project_root: &std::path::Path) {
    std::fs::create_dir_all(project_root.join("src")).expect("create src");
    std::fs::write(
        project_root.join("src/lib.rs"),
        (0..320)
            .map(|idx| format!("pub const LINE_{idx}: &str = \"value_{idx}\";\n"))
            .collect::<String>(),
    )
    .expect("write source");
}

fn run_quality_snapshot(project_root: &std::path::Path, args: &[&str]) -> Value {
    let assert = cargo_bin_cmd!("rmu-cli")
        .arg("--project-path")
        .arg(project_root.to_str().expect("utf-8 project path"))
        .arg("--json")
        .arg("quality-snapshot")
        .args(args)
        .assert()
        .success();
    serde_json::from_slice(&assert.get_output().stdout).expect("json stdout")
}

#[test]
fn quality_snapshot_persists_wave_history_and_self_baseline() {
    let project = tempdir().expect("tempdir");
    write_hot_file(project.path());
    let custom_output_root = project.path().join("tmp-quality-output");

    let before = run_quality_snapshot(
        project.path(),
        &[
            "--snapshot-kind",
            "before",
            "--wave-id",
            "wave-0",
            "--output-root",
            custom_output_root.to_str().expect("utf-8 output root"),
        ],
    );
    let before_root = std::fs::read_dir(custom_output_root.join("quality-waves/wave-0/before"))
        .expect("before dir")
        .next()
        .expect("before entry")
        .expect("before dir entry")
        .path();
    assert!(before_root.join("snapshot.report.json").exists());
    assert_eq!(before["snapshot"]["snapshot_kind"], serde_json::json!("before"));

    let after = run_quality_snapshot(
        project.path(),
        &[
            "--snapshot-kind",
            "after",
            "--wave-id",
            "wave-0",
            "--output-root",
            custom_output_root.to_str().expect("utf-8 output root"),
            "--compare-against",
            "wave_before",
            "--promote-self-baseline",
        ],
    );

    assert_eq!(after["delta"]["gate_status"], serde_json::json!("ok"));
    assert_eq!(after["delta"]["new_violations"], serde_json::json!(0));
    assert_eq!(after["delta"]["resolved_violations"], serde_json::json!(0));
    assert!(
        project
            .path()
            .join("baseline/quality/self/baseline-summary.json")
            .exists()
    );
    assert!(
        project
            .path()
            .join("tmp-quality-output/quality-waves/wave-0/delta")
            .read_dir()
            .expect("delta dir")
            .next()
            .is_some()
    );
}
