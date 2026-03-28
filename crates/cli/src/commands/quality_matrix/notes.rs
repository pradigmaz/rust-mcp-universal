use std::collections::BTreeSet;

use serde_json::Value;

use super::report::QualityMatrixRepoReport;

pub(super) fn notes_markdown(
    repo: &QualityMatrixRepoReport,
    evaluated_files: usize,
    violating_files: usize,
    total_violations: usize,
    duplication_clone_classes: Option<&Value>,
    duplication_policy_present: bool,
) -> String {
    let suppression_summary = summarize_duplication_suppressions(duplication_clone_classes);
    format!(
        "# Quality Matrix Notes: {repo_id}\n\npre_refresh_status={pre}\npost_refresh_status={post}\n\
evaluated_files={evaluated}\nviolating_files={violating}\ntotal_violations={violations}\n\
languages_match={languages_match}\npre_status_match={pre_match}\npost_status_match={post_match}\n\
duplication_policy_present={duplication_policy_present}\nsuppressed_clone_classes={suppressed_clone_classes}\n\
duplication_suppression_ids={duplication_suppression_ids}\nreview_shortlist={review_shortlist}\nmanual_review_required={manual_review_required}\n",
        repo_id = repo.repo_id,
        pre = repo.pre_refresh_status,
        post = repo.post_refresh_status,
        evaluated = evaluated_files,
        violating = violating_files,
        violations = total_violations,
        languages_match = repo.validations.languages_match,
        pre_match = repo.validations.pre_status_match,
        post_match = repo.validations.post_status_match,
        duplication_policy_present = duplication_policy_present,
        suppressed_clone_classes = suppression_summary.suppressed_clone_classes,
        duplication_suppression_ids = suppression_summary.suppression_ids.join(", "),
        review_shortlist = repo.noise_summary.review_shortlist.join(", "),
        manual_review_required = repo.noise_summary.manual_review_required
    )
}

#[derive(Debug, Default, PartialEq, Eq)]
struct DuplicationSuppressionSummary {
    suppressed_clone_classes: usize,
    suppression_ids: Vec<String>,
}

fn summarize_duplication_suppressions(
    duplication_clone_classes: Option<&Value>,
) -> DuplicationSuppressionSummary {
    let Some(artifact) = duplication_clone_classes else {
        return DuplicationSuppressionSummary::default();
    };
    let Some(suppressed) = artifact
        .get("suppressed_clone_classes")
        .and_then(Value::as_array)
    else {
        return DuplicationSuppressionSummary::default();
    };

    let mut suppression_ids = BTreeSet::new();
    for entry in suppressed {
        let Some(suppressions) = entry.get("suppressions").and_then(Value::as_array) else {
            continue;
        };
        for suppression in suppressions {
            let Some(id) = suppression.get("suppression_id").and_then(Value::as_str) else {
                continue;
            };
            if !id.is_empty() {
                suppression_ids.insert(id.to_string());
            }
        }
    }

    DuplicationSuppressionSummary {
        suppressed_clone_classes: suppressed.len(),
        suppression_ids: suppression_ids.into_iter().collect(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{DuplicationSuppressionSummary, summarize_duplication_suppressions};

    #[test]
    fn summarize_duplication_suppressions_handles_missing_artifact() {
        assert_eq!(
            summarize_duplication_suppressions(None),
            DuplicationSuppressionSummary::default()
        );
    }

    #[test]
    fn summarize_duplication_suppressions_collects_unique_sorted_ids() {
        let artifact = json!({
            "suppressed_clone_classes": [
                {
                    "suppressions": [
                        {"suppression_id": "beta"},
                        {"suppression_id": "alpha"}
                    ]
                },
                {
                    "suppressions": [
                        {"suppression_id": "beta"},
                        {"suppression_id": "gamma"}
                    ]
                }
            ]
        });

        assert_eq!(
            summarize_duplication_suppressions(Some(&artifact)),
            DuplicationSuppressionSummary {
                suppressed_clone_classes: 2,
                suppression_ids: vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string(),],
            }
        );
    }
}
