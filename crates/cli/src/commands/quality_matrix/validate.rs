use std::collections::BTreeSet;

use anyhow::{Result, bail};
use rmu_core::{Engine, QualityStatus, WorkspaceBrief};

use super::manifest;
use super::report;

pub(super) fn validate_repo(
    repo: &manifest::ResolvedQualityMatrixRepository,
    engine: &Engine,
    brief_before_refresh: &WorkspaceBrief,
    brief_after_refresh: &WorkspaceBrief,
) -> Result<report::QualityMatrixValidations> {
    let validations = report::QualityMatrixValidations {
        languages_match: languages_match(repo, brief_after_refresh),
        pre_status_match: repo
            .config
            .expected_pre_refresh_statuses
            .contains(&brief_before_refresh.quality_summary.status),
        post_status_match: repo
            .config
            .expected_post_refresh_statuses
            .contains(&brief_after_refresh.quality_summary.status),
    };

    if !validations.pre_status_match {
        bail!(
            "repo `{}` produced unexpected pre-refresh status `{}`",
            repo.config.id,
            brief_before_refresh.quality_summary.status.as_str()
        );
    }
    if !validations.post_status_match {
        bail!(
            "repo `{}` produced unexpected post-refresh status `{}`",
            repo.config.id,
            brief_after_refresh.quality_summary.status.as_str()
        );
    }
    if !validations.languages_match {
        bail!(
            "repo `{}` produced languages that do not match manifest expectations",
            repo.config.id
        );
    }
    validate_degradation_reason(repo, engine, brief_after_refresh)?;

    Ok(validations)
}

fn validate_degradation_reason(
    repo: &manifest::ResolvedQualityMatrixRepository,
    engine: &Engine,
    brief_after_refresh: &WorkspaceBrief,
) -> Result<()> {
    if brief_after_refresh.quality_summary.status != QualityStatus::Degraded {
        return Ok(());
    }

    let Some(reason) = engine.quality_degradation_reason()? else {
        bail!(
            "repo `{}` finished degraded without a recorded quality degradation reason",
            repo.config.id
        );
    };
    if repo
        .config
        .allowed_degradation_reasons
        .iter()
        .any(|allowed| allowed == &reason)
    {
        return Ok(());
    }

    bail!(
        "repo `{}` finished degraded for disallowed reason `{}`",
        repo.config.id,
        reason
    );
}

fn languages_match(
    repo: &manifest::ResolvedQualityMatrixRepository,
    brief_after_refresh: &WorkspaceBrief,
) -> bool {
    let observed = brief_after_refresh
        .languages
        .iter()
        .map(|entry| entry.language.to_ascii_lowercase())
        .collect::<BTreeSet<_>>();

    repo.config
        .expected_languages
        .iter()
        .all(|language| observed.contains(&language.to_ascii_lowercase()))
}
