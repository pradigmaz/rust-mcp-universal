use serde_json::json;

use super::derive_duplication_review_shortlist;

#[test]
fn review_shortlist_uses_duplication_artifact_instead_of_generic_violation_order() {
    let artifact = json!({
        "clone_classes": [
            {
                "clone_class_id": "small",
                "corpus_class": "production",
                "normalized_token_count": 50,
                "similarity_percent": 100,
                "cross_file": true,
                "members": [
                    {"path": "src/small_a.rs"},
                    {"path": "src/small_b.rs"}
                ]
            },
            {
                "clone_class_id": "large",
                "corpus_class": "production",
                "normalized_token_count": 150,
                "similarity_percent": 90,
                "cross_file": true,
                "members": [
                    {"path": "src/large_a.rs"},
                    {"path": "src/large_b.rs"},
                    {"path": "src/large_c.rs"}
                ]
            },
            {
                "clone_class_id": "boilerplate",
                "corpus_class": "production",
                "normalized_token_count": 999,
                "similarity_percent": 100,
                "cross_file": true,
                "signal_role": "boilerplate",
                "members": [
                    {"path": "src/ignored_a.rs"},
                    {"path": "src/ignored_b.rs"}
                ]
            },
            {
                "clone_class_id": "test-surface",
                "corpus_class": "test",
                "normalized_token_count": 500,
                "similarity_percent": 100,
                "cross_file": true,
                "members": [
                    {"path": "tests/ignored_a.rs"},
                    {"path": "tests/ignored_b.rs"}
                ]
            }
        ]
    });

    let shortlist =
        derive_duplication_review_shortlist(Some(&artifact), 5).expect("review shortlist");

    assert_eq!(
        shortlist,
        vec![
            "src/large_a.rs".to_string(),
            "src/large_b.rs".to_string(),
            "src/large_c.rs".to_string(),
            "src/small_a.rs".to_string(),
            "src/small_b.rs".to_string(),
        ]
    );
}

#[test]
fn review_shortlist_is_empty_without_unsuppressed_production_cross_file_duplicates() {
    let artifact = json!({
        "clone_classes": [
            {
                "clone_class_id": "same-file",
                "corpus_class": "production",
                "normalized_token_count": 150,
                "similarity_percent": 100,
                "cross_file": false,
                "members": [
                    {"path": "src/lib.rs"}
                ]
            }
        ]
    });

    let shortlist =
        derive_duplication_review_shortlist(Some(&artifact), 5).expect("review shortlist");
    assert!(shortlist.is_empty());
}
