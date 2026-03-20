use anyhow::{Result, bail};

pub(crate) fn require_min(field: &str, value: usize, minimum: usize) -> Result<usize> {
    if value < minimum {
        bail!("`{field}` must be >= {minimum}, got {value}");
    }
    Ok(value)
}

pub(crate) fn require_max(field: &str, value: usize, maximum: usize) -> Result<usize> {
    if value > maximum {
        bail!("`{field}` must be <= {maximum}, got {value}");
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::{require_max, require_min};

    #[test]
    fn require_min_rejects_small_values() {
        let err = require_min("limit", 0, 1).expect_err("must reject value below minimum");
        assert!(err.to_string().contains("`limit` must be >= 1, got 0"));
    }

    #[test]
    fn require_max_rejects_large_values() {
        let err = require_max("limit", 10, 9).expect_err("must reject value above maximum");
        assert!(err.to_string().contains("`limit` must be <= 9, got 10"));
    }
}
