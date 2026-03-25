use std::collections::BTreeSet;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root")
}

fn read_json(path: &PathBuf) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(path).expect("read json")).expect("parse json")
}

#[test]
fn stage9_gold_datasets_cover_aggregate_dataset_without_case_drift() {
    let root = workspace_root();
    let aggregate_path = root.join("baseline/investigation/stage9/investigation_dataset.json");
    let aggregate = read_json(&aggregate_path);
    let aggregate_ids = aggregate["cases"]
        .as_array()
        .expect("aggregate cases")
        .iter()
        .map(|case| case["id"].as_str().expect("aggregate case id").to_string())
        .collect::<BTreeSet<_>>();

    let gold_paths = [
        root.join("baseline/investigation/stage9/gold/symbol_body_dataset.json"),
        root.join("baseline/investigation/stage9/gold/route_trace_dataset.json"),
        root.join("baseline/investigation/stage9/gold/constraint_evidence_dataset.json"),
        root.join("baseline/investigation/stage9/gold/concept_cluster_dataset.json"),
        root.join("baseline/investigation/stage9/gold/divergence_report_dataset.json"),
    ];
    let union_ids = gold_paths
        .iter()
        .flat_map(|path| {
            read_json(path)["cases"]
                .as_array()
                .expect("gold cases")
                .iter()
                .map(|case| case["id"].as_str().expect("gold case id").to_string())
                .collect::<Vec<_>>()
        })
        .collect::<BTreeSet<_>>();

    assert_eq!(aggregate_ids, union_ids);
}

#[test]
fn stage9_reports_match_aggregate_dataset_case_inventory() {
    let root = workspace_root();
    let aggregate =
        read_json(&root.join("baseline/investigation/stage9/investigation_dataset.json"));
    let latest_report = read_json(&root.join("baseline/investigation/stage9/latest_report.json"));
    let baseline_report =
        read_json(&root.join("baseline/investigation/stage9/baseline_report.json"));

    let aggregate_ids = aggregate["cases"]
        .as_array()
        .expect("aggregate cases")
        .iter()
        .map(|case| {
            (
                case["id"].as_str().expect("case id").to_string(),
                case["expected_capability_status"]
                    .as_str()
                    .expect("expected capability status")
                    .to_string(),
            )
        })
        .collect::<BTreeSet<_>>();

    for report in [&latest_report, &baseline_report] {
        assert_eq!(
            report["case_count"].as_u64().expect("report case_count") as usize,
            aggregate_ids.len()
        );
        let report_ids = report["cases"]
            .as_array()
            .expect("report cases")
            .iter()
            .map(|case| {
                (
                    case["id"].as_str().expect("case id").to_string(),
                    case["expected_capability_status"]
                        .as_str()
                        .expect("expected capability status")
                        .to_string(),
                )
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(report_ids, aggregate_ids);
    }
}
