use crate::utils::decode_normalized_path;

pub(super) fn normalize(raw: &str) -> String {
    let normalized = raw
        .replace('\\', "/")
        .trim()
        .trim_start_matches("./")
        .trim_start_matches('/')
        .to_string();
    #[cfg(windows)]
    {
        normalized.to_lowercase()
    }
    #[cfg(not(windows))]
    {
        normalized
    }
}

pub(super) fn normalize_match_candidates(raw: &str) -> Vec<String> {
    let normalized = normalize(raw);
    let mut out = vec![normalized.clone()];
    if let Some(decoded) = decode_scope_path(&normalized) {
        let decoded_norm = normalize(&decoded);
        if decoded_norm != normalized {
            out.push(decoded_norm);
        }
    }
    out
}

#[cfg(windows)]
pub(super) fn decode_scope_path(input: &str) -> Option<String> {
    decode_normalized_path(input)
}

#[cfg(not(windows))]
pub(super) fn decode_scope_path(input: &str) -> Option<String> {
    decode_normalized_path(input)
}
