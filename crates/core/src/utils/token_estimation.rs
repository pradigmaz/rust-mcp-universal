const TOKEN_MODEL_ENV: &str = "RMU_TOKEN_ESTIMATOR_MODEL";
const TOKEN_ASCII_CPT_ENV: &str = "RMU_TOKEN_ASCII_CHARS_PER_TOKEN";
const TOKEN_NON_ASCII_CPT_ENV: &str = "RMU_TOKEN_NON_ASCII_CHARS_PER_TOKEN";
const TOKEN_PUNCT_WEIGHT_ENV: &str = "RMU_TOKEN_PUNCT_WEIGHT";
const TOKEN_NEWLINE_WEIGHT_ENV: &str = "RMU_TOKEN_NEWLINE_WEIGHT";

pub fn estimate_tokens_for_text(text: &str) -> usize {
    let policy = TokenEstimatePolicy::from_env();
    estimate_tokens_for_text_with_policy(text, policy)
}

#[derive(Debug, Clone, Copy)]
struct TokenEstimatePolicy {
    ascii_chars_per_token: f32,
    non_ascii_chars_per_token: f32,
    punctuation_weight: f32,
    newline_weight: f32,
}

impl TokenEstimatePolicy {
    fn from_env() -> Self {
        let preset = std::env::var(TOKEN_MODEL_ENV).unwrap_or_else(|_| "cl100k".to_string());
        let mut policy = Self::for_model(preset.as_str());
        if let Some(value) = parse_positive_f32_env(TOKEN_ASCII_CPT_ENV) {
            policy.ascii_chars_per_token = value;
        }
        if let Some(value) = parse_positive_f32_env(TOKEN_NON_ASCII_CPT_ENV) {
            policy.non_ascii_chars_per_token = value;
        }
        if let Some(value) = parse_positive_f32_env(TOKEN_PUNCT_WEIGHT_ENV) {
            policy.punctuation_weight = value;
        }
        if let Some(value) = parse_positive_f32_env(TOKEN_NEWLINE_WEIGHT_ENV) {
            policy.newline_weight = value;
        }
        policy
    }

    fn for_model(name: &str) -> Self {
        match name.trim().to_ascii_lowercase().as_str() {
            "o200k" | "gpt-4o" | "gpt4o" => Self {
                ascii_chars_per_token: 4.1,
                non_ascii_chars_per_token: 1.7,
                punctuation_weight: 0.28,
                newline_weight: 0.16,
            },
            "claude" | "claude3" | "claude-3" => Self {
                ascii_chars_per_token: 3.6,
                non_ascii_chars_per_token: 1.45,
                punctuation_weight: 0.31,
                newline_weight: 0.18,
            },
            "legacy" => Self {
                ascii_chars_per_token: 4.0,
                non_ascii_chars_per_token: 4.0,
                punctuation_weight: 0.0,
                newline_weight: 0.0,
            },
            _ => Self {
                ascii_chars_per_token: 3.8,
                non_ascii_chars_per_token: 1.55,
                punctuation_weight: 0.30,
                newline_weight: 0.17,
            },
        }
    }
}

fn parse_positive_f32_env(var: &str) -> Option<f32> {
    let raw = std::env::var(var).ok()?;
    let parsed = raw.parse::<f32>().ok()?;
    if parsed.is_finite() && parsed > 0.0 {
        Some(parsed)
    } else {
        None
    }
}

fn estimate_tokens_for_text_with_policy(text: &str, policy: TokenEstimatePolicy) -> usize {
    if text.is_empty() {
        return 0;
    }

    let mut ascii_alnum = 0_usize;
    let mut ascii_symbols = 0_usize;
    let mut non_ascii = 0_usize;
    let mut punctuation = 0_usize;
    let mut newlines = 0_usize;

    for ch in text.chars() {
        if ch == '\n' {
            newlines += 1;
        }
        if ch.is_ascii_alphanumeric() || ch == '_' {
            ascii_alnum += 1;
            continue;
        }
        if ch.is_ascii() {
            if ch.is_ascii_punctuation() {
                punctuation += 1;
            }
            if !ch.is_whitespace() {
                ascii_symbols += 1;
            }
            continue;
        }
        if !ch.is_whitespace() {
            non_ascii += 1;
        }
    }

    let ascii_tokens = ascii_alnum as f32 / policy.ascii_chars_per_token.max(0.25);
    let symbol_tokens = ascii_symbols as f32 * 0.55;
    let non_ascii_tokens = non_ascii as f32 / policy.non_ascii_chars_per_token.max(0.25);
    let punctuation_tokens = punctuation as f32 * policy.punctuation_weight;
    let newline_tokens = newlines as f32 * policy.newline_weight;

    let estimate =
        (ascii_tokens + symbol_tokens + non_ascii_tokens + punctuation_tokens + newline_tokens)
            .ceil();
    if estimate <= 0.0 {
        1
    } else {
        usize::try_from(estimate as u64).unwrap_or(usize::MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::{TokenEstimatePolicy, estimate_tokens_for_text_with_policy};

    #[test]
    fn model_aware_token_estimation_increases_for_code_punctuation() {
        let plain = "alpha beta gamma delta epsilon";
        let code = "fn alpha_beta(x: i32) -> i32 {\n    x + 1\n}\n";
        let policy = TokenEstimatePolicy::for_model("cl100k");
        let plain_tokens = estimate_tokens_for_text_with_policy(plain, policy);
        let code_tokens = estimate_tokens_for_text_with_policy(code, policy);
        assert!(code_tokens > plain_tokens);
    }

    #[test]
    fn non_ascii_text_is_not_collapsed_by_ascii_ratio() {
        let policy = TokenEstimatePolicy::for_model("cl100k");
        let latin = estimate_tokens_for_text_with_policy("hello world", policy);
        let cjk = estimate_tokens_for_text_with_policy("你好世界你好世界", policy);
        assert!(cjk >= latin);
    }

    #[test]
    fn legacy_policy_matches_chars_div_4_shape() {
        let legacy = TokenEstimatePolicy::for_model("legacy");
        let text = "abcdefgh";
        let estimate = estimate_tokens_for_text_with_policy(text, legacy);
        assert_eq!(estimate, 2);
    }
}
