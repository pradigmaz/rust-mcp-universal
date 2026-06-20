use anyhow::{Context, Result, bail};

pub(super) fn require_non_empty<'a>(value: &'a str, field: &str) -> Result<&'a str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        bail!("`{field}` must be non-empty");
    }
    Ok(trimmed)
}

pub(super) fn db_limit(limit: usize, field: &str) -> Result<i64> {
    if limit == 0 {
        bail!("`{field}` must be >= 1");
    }
    i64::try_from(limit).with_context(|| {
        format!(
            "`{field}` value {limit} exceeds maximum supported value {}",
            i64::MAX
        )
    })
}
