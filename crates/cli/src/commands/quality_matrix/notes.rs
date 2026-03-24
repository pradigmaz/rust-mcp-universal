use super::report::QualityMatrixRepoReport;

pub(super) fn notes_markdown(
    repo: &QualityMatrixRepoReport,
    evaluated_files: usize,
    violating_files: usize,
    total_violations: usize,
) -> String {
    format!(
        "# Quality Matrix Notes: {repo_id}\n\npre_refresh_status={pre}\npost_refresh_status={post}\n\
evaluated_files={evaluated}\nviolating_files={violating}\ntotal_violations={violations}\n\
languages_match={languages_match}\npre_status_match={pre_match}\npost_status_match={post_match}\n\
review_shortlist={review_shortlist}\nmanual_review_required=true\n",
        repo_id = repo.repo_id,
        pre = repo.pre_refresh_status,
        post = repo.post_refresh_status,
        evaluated = evaluated_files,
        violating = violating_files,
        violations = total_violations,
        languages_match = repo.validations.languages_match,
        pre_match = repo.validations.pre_status_match,
        post_match = repo.validations.post_status_match,
        review_shortlist = repo.noise_summary.review_shortlist.join(", ")
    )
}
