use anyhow::Result;

use crate::model::IndexingOptions;

mod glob;
mod normalize;

#[cfg(test)]
mod tests;

const MAX_GLOB_VARIANTS: usize = 4096;
const MAX_EXTGLOB_ALTERNATIVES: usize = 256;
const MAX_EXTGLOB_NESTING: usize = 32;
const MAX_SEGMENT_TOKENS: usize = 512;
const MAX_MATCH_TEXT_CHARS: usize = 1024;
const MAX_MATCH_STATE_CELLS: usize = 300_000;

#[derive(Debug, Clone)]
enum ScopeRule {
    Path(String),
    Glob(Vec<String>),
}

impl ScopeRule {
    fn parse(raw: &str) -> Result<Option<Self>> {
        let pattern = normalize::normalize(raw);
        if pattern.is_empty() {
            return Ok(None);
        }

        if glob::has_glob_meta(&pattern) {
            let expanded = glob::expand_braces(&pattern)?;
            for variant in &expanded {
                glob::validate_glob_variant(variant)?;
            }
            return Ok(Some(Self::Glob(expanded)));
        }

        let normalized = pattern.trim_end_matches('/').to_string();
        if normalized.is_empty() {
            return Ok(None);
        }

        Ok(Some(Self::Path(normalized)))
    }

    fn matches(&self, path: &str) -> bool {
        let path_variants = normalize::normalize_match_candidates(path);
        match self {
            Self::Path(root) => path_variants
                .iter()
                .any(|candidate| candidate == root || candidate.starts_with(&format!("{root}/"))),
            Self::Glob(patterns) => patterns.iter().any(|pattern| {
                path_variants
                    .iter()
                    .any(|candidate| glob::glob_match_single_variant(pattern, candidate))
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct IndexScope {
    profile_includes: Vec<ScopeRule>,
    manual_includes: Vec<ScopeRule>,
    excludes: Vec<ScopeRule>,
}

impl IndexScope {
    pub(crate) fn new(options: &IndexingOptions) -> Result<Self> {
        let profile_includes = parse_rules(
            options
                .profile
                .into_iter()
                .flat_map(|profile| profile.include_paths().iter().copied()),
        )?;
        let manual_includes = parse_rules(options.include_paths.iter().map(String::as_str))?;
        let mut excludes = parse_rules(options.exclude_paths.iter().map(String::as_str))?;
        excludes.extend(parse_rules(
            options
                .profile
                .into_iter()
                .flat_map(|profile| profile.exclude_paths().iter().copied()),
        )?);

        Ok(Self {
            profile_includes,
            manual_includes,
            excludes,
        })
    }

    pub(crate) fn has_rules(&self) -> bool {
        !self.profile_includes.is_empty()
            || !self.manual_includes.is_empty()
            || !self.excludes.is_empty()
    }

    pub(crate) fn allows(&self, raw_path: &str) -> bool {
        let path = normalize::normalize(raw_path);
        let profile_allowed = if self.profile_includes.is_empty() {
            true
        } else {
            self.profile_includes.iter().any(|rule| rule.matches(&path))
        };
        if !profile_allowed {
            return false;
        }

        let manual_allowed = if self.manual_includes.is_empty() {
            true
        } else {
            self.manual_includes.iter().any(|rule| rule.matches(&path))
        };
        if !manual_allowed {
            return false;
        }

        !self.excludes.iter().any(|rule| rule.matches(&path))
    }
}

fn parse_rules<'a>(values: impl IntoIterator<Item = &'a str>) -> Result<Vec<ScopeRule>> {
    values
        .into_iter()
        .filter_map(|value| ScopeRule::parse(value).transpose())
        .collect::<Result<Vec<_>>>()
}
