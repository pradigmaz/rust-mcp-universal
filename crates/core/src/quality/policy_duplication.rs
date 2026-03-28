use std::collections::BTreeSet;

use anyhow::Result;

use crate::model::QualitySuppression;
use crate::quality::duplication::artifact::DuplicationCloneClass;
use crate::quality::policy_schema::{
    DuplicationPathPairFile, DuplicationPolicyFile, DuplicationSuppressionFile,
};

use super::PathMatcher;

#[derive(Debug, Clone, Default)]
pub(crate) struct DuplicationPolicy {
    pub(crate) suppressions: Vec<DuplicationSuppressionPolicy>,
}

#[derive(Debug, Clone)]
pub(crate) struct DuplicationSuppressionPolicy {
    pub(crate) suppression_id: String,
    pub(crate) reason: String,
    pub(crate) scope_id: Option<String>,
    pub(crate) scope_matcher: Option<PathMatcher>,
    clone_class_ids: BTreeSet<String>,
    path_pairs: Vec<DuplicationPathPairPolicy>,
}

#[derive(Debug, Clone)]
struct DuplicationPathPairPolicy {
    left: PathMatcher,
    right: PathMatcher,
}

impl DuplicationPolicy {
    pub(crate) fn suppressions_for_class(
        &self,
        class: &DuplicationCloneClass,
    ) -> Vec<QualitySuppression> {
        let member_paths = class
            .members
            .iter()
            .map(|member| member.path.as_str())
            .collect::<BTreeSet<_>>();
        let mut matches = self
            .suppressions
            .iter()
            .filter(|suppression| suppression.matches(class, &member_paths))
            .map(|suppression| QualitySuppression {
                suppression_id: suppression.suppression_id.clone(),
                reason: suppression.reason.clone(),
                scope_id: suppression.scope_id.clone(),
            })
            .collect::<Vec<_>>();
        matches.sort_by(|left, right| left.suppression_id.cmp(&right.suppression_id));
        matches
    }

    pub(crate) fn extend_from(&mut self, other: Self) {
        self.suppressions.extend(other.suppressions);
    }
}

impl DuplicationSuppressionPolicy {
    fn matches(&self, class: &DuplicationCloneClass, member_paths: &BTreeSet<&str>) -> bool {
        if let Some(scope_matcher) = &self.scope_matcher {
            if !member_paths.iter().any(|path| scope_matcher.matches(path)) {
                return false;
            }
        }
        self.clone_class_ids.contains(&class.clone_class_id)
            || self
                .path_pairs
                .iter()
                .any(|pair| pair.matches(member_paths))
    }
}

impl DuplicationPathPairPolicy {
    fn matches(&self, member_paths: &BTreeSet<&str>) -> bool {
        for left in member_paths {
            for right in member_paths {
                if left == right {
                    continue;
                }
                if self.left.matches(left) && self.right.matches(right) {
                    return true;
                }
                if self.left.matches(right) && self.right.matches(left) {
                    return true;
                }
            }
        }
        false
    }
}

pub(crate) fn duplication_policy_from_file(
    scope_id: Option<&str>,
    scope_paths: Option<&[String]>,
    parsed: Option<DuplicationPolicyFile>,
) -> Result<DuplicationPolicy> {
    let Some(parsed) = parsed else {
        return Ok(DuplicationPolicy::default());
    };
    let scope_matcher = match scope_paths {
        Some(paths) if !paths.is_empty() => Some(PathMatcher::new(paths)?),
        _ => None,
    };
    Ok(DuplicationPolicy {
        suppressions: parsed
            .suppressions
            .into_iter()
            .map(|suppression| {
                duplication_suppression_from_file(scope_id, scope_matcher.clone(), suppression)
            })
            .collect::<Result<Vec<_>>>()?,
    })
}

fn duplication_suppression_from_file(
    scope_id: Option<&str>,
    scope_matcher: Option<PathMatcher>,
    parsed: DuplicationSuppressionFile,
) -> Result<DuplicationSuppressionPolicy> {
    Ok(DuplicationSuppressionPolicy {
        suppression_id: parsed.id,
        reason: parsed.reason,
        scope_id: scope_id.map(str::to_string),
        scope_matcher,
        clone_class_ids: parsed.clone_class_ids.into_iter().collect(),
        path_pairs: parsed
            .path_pairs
            .into_iter()
            .map(path_pair_from_file)
            .collect::<Result<Vec<_>>>()?,
    })
}

fn path_pair_from_file(parsed: DuplicationPathPairFile) -> Result<DuplicationPathPairPolicy> {
    Ok(DuplicationPathPairPolicy {
        left: PathMatcher::new(&[parsed.left])?,
        right: PathMatcher::new(&[parsed.right])?,
    })
}
