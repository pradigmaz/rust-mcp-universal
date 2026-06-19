use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;

use crate::model::{
    QualityCategory, QualitySeverity, RuleViolationFileHit, WorkspaceQualityCategoryCount,
    WorkspaceQualitySeverityCount,
};

pub(super) fn load_severity_breakdown(
    conn: &rusqlite::Connection,
) -> Result<Vec<WorkspaceQualitySeverityCount>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT severity, COUNT(DISTINCT path) AS file_count, COUNT(1) AS violation_count
        FROM file_rule_violations
        GROUP BY severity
        ORDER BY violation_count DESC, severity ASC
        "#,
    )?;
    stmt.query_map([], |row| {
        Ok(WorkspaceQualitySeverityCount {
            severity: row
                .get::<_, String>(0)
                .ok()
                .and_then(|value| QualitySeverity::parse(&value))
                .unwrap_or(QualitySeverity::Medium),
            files: usize::try_from(row.get::<_, i64>(1)?).unwrap_or(usize::MAX),
            violations: usize::try_from(row.get::<_, i64>(2)?).unwrap_or(usize::MAX),
        })
    })?
    .collect::<rusqlite::Result<Vec<_>>>()
    .map_err(Into::into)
}

pub(super) fn load_category_breakdown(
    conn: &rusqlite::Connection,
) -> Result<Vec<WorkspaceQualityCategoryCount>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT category, COUNT(DISTINCT path) AS file_count, COUNT(1) AS violation_count
        FROM file_rule_violations
        GROUP BY category
        ORDER BY violation_count DESC, category ASC
        "#,
    )?;
    stmt.query_map([], |row| {
        Ok(WorkspaceQualityCategoryCount {
            category: row
                .get::<_, String>(0)
                .ok()
                .and_then(|value| QualityCategory::parse(&value))
                .unwrap_or(QualityCategory::Maintainability),
            files: usize::try_from(row.get::<_, i64>(1)?).unwrap_or(usize::MAX),
            violations: usize::try_from(row.get::<_, i64>(2)?).unwrap_or(usize::MAX),
        })
    })?
    .collect::<rusqlite::Result<Vec<_>>>()
    .map_err(Into::into)
}

pub(super) fn build_severity_breakdown(
    hits: &[RuleViolationFileHit],
) -> Vec<WorkspaceQualitySeverityCount> {
    let mut counts = BTreeMap::<QualitySeverity, (usize, BTreeSet<String>)>::new();
    for hit in hits {
        for violation in &hit.violations {
            let entry = counts
                .entry(violation.severity)
                .or_insert_with(|| (0, BTreeSet::new()));
            entry.0 += 1;
            entry.1.insert(hit.path.clone());
        }
    }
    counts
        .into_iter()
        .map(
            |(severity, (violations, files))| WorkspaceQualitySeverityCount {
                severity,
                files: files.len(),
                violations,
            },
        )
        .collect()
}

pub(super) fn build_category_breakdown(
    hits: &[RuleViolationFileHit],
) -> Vec<WorkspaceQualityCategoryCount> {
    let mut counts = BTreeMap::<QualityCategory, (usize, BTreeSet<String>)>::new();
    for hit in hits {
        for violation in &hit.violations {
            let entry = counts
                .entry(violation.category)
                .or_insert_with(|| (0, BTreeSet::new()));
            entry.0 += 1;
            entry.1.insert(hit.path.clone());
        }
    }
    counts
        .into_iter()
        .map(
            |(category, (violations, files))| WorkspaceQualityCategoryCount {
                category,
                files: files.len(),
                violations,
            },
        )
        .collect()
}
