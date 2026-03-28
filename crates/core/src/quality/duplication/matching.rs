use super::tokenize::NormalizedToken;

const RESYNC_LOOKAHEAD: usize = 12;
const MIN_SIMILARITY_PERCENT: usize = 85;
const MAX_EDIT_RATIO_BPS: usize = 1_500;
const MAX_EDIT_SLACK: usize = 6;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ApproximateMatch {
    pub(crate) left_start: usize,
    pub(crate) right_start: usize,
    pub(crate) left_len: usize,
    pub(crate) right_len: usize,
    pub(crate) common_tokens: usize,
    pub(crate) similarity_percent: i64,
    pub(crate) signature_tokens: Vec<String>,
}

pub(crate) fn approximate_match_from_anchor(
    left: &[NormalizedToken],
    left_anchor: usize,
    right: &[NormalizedToken],
    right_anchor: usize,
    anchor_len: usize,
) -> Option<ApproximateMatch> {
    let mut left_start = left_anchor;
    let mut right_start = right_anchor;
    while left_start > 0
        && right_start > 0
        && left[left_start - 1].value == right[right_start - 1].value
    {
        left_start -= 1;
        right_start -= 1;
    }
    let (left_len, right_len, common_tokens, similarity_percent, edit_count, signature_tokens) =
        extend_forward_with_tolerance(&left[left_start..], &right[right_start..], anchor_len);
    if common_tokens < anchor_len
        || similarity_percent >= 100
        || similarity_percent < MIN_SIMILARITY_PERCENT as i64
        || edit_count == 0
    {
        return None;
    }
    Some(ApproximateMatch {
        left_start,
        right_start,
        left_len,
        right_len,
        common_tokens,
        similarity_percent,
        signature_tokens,
    })
}

fn extend_forward_with_tolerance(
    left: &[NormalizedToken],
    right: &[NormalizedToken],
    anchor_len: usize,
) -> (usize, usize, usize, i64, usize, Vec<String>) {
    let mut left_idx = 0;
    let mut right_idx = 0;
    let mut common_tokens = 0;
    let mut edits = 0;
    let mut signature_tokens = Vec::new();

    while left_idx < left.len() && right_idx < right.len() {
        if left[left_idx].value == right[right_idx].value {
            signature_tokens.push(left[left_idx].value.clone());
            left_idx += 1;
            right_idx += 1;
            common_tokens += 1;
            continue;
        }

        let allowed_edits =
            ((common_tokens.max(anchor_len) * MAX_EDIT_RATIO_BPS) / 10_000) + MAX_EDIT_SLACK;
        if edits >= allowed_edits {
            break;
        }

        if left
            .get(left_idx + 1)
            .zip(right.get(right_idx + 1))
            .is_some_and(|(next_left, next_right)| next_left.value == next_right.value)
        {
            edits += 1;
            left_idx += 1;
            right_idx += 1;
            continue;
        }

        if let Some(skip) = resync_skip(&left[left_idx..], &right[right_idx].value) {
            edits += skip;
            left_idx += skip;
            continue;
        }
        if let Some(skip) = resync_skip(&right[right_idx..], &left[left_idx].value) {
            edits += skip;
            right_idx += skip;
            continue;
        }

        edits += 1;
        left_idx += 1;
        right_idx += 1;
    }

    let span = left_idx.max(right_idx).max(1);
    let similarity_percent = ((common_tokens * 100) / span) as i64;
    (
        left_idx,
        right_idx,
        common_tokens,
        similarity_percent,
        edits,
        signature_tokens,
    )
}

fn resync_skip(tokens: &[NormalizedToken], needle: &str) -> Option<usize> {
    (1..=RESYNC_LOOKAHEAD).find(|offset| {
        tokens
            .get(*offset)
            .is_some_and(|token| token.value == needle)
    })
}

#[cfg(test)]
mod tests {
    use super::{ApproximateMatch, approximate_match_from_anchor};
    use crate::quality::duplication::tokenize::{NormalizedToken, normalize_tokens};

    #[test]
    fn approximate_match_rejects_exact_clones() {
        let left = token_stream(&[
            "fn", "$id", "(", ")", "{", "$id", "+=", "$num", ";", "$id", "+=", "$num", ";", "$id",
            "+=", "$num", ";", "$id", "+=", "$num", ";", "$id", "+=", "$num", ";", "$id", "+=",
            "$num", ";", "$id", "+=", "$num", ";", "$id", "+=", "$num", ";", "return", "$id", ";",
            "}",
        ]);
        let matched = approximate_match_from_anchor(&left, 0, &left, 0, 32);
        assert_eq!(matched, None::<ApproximateMatch>);
    }

    #[test]
    fn approximate_match_accepts_strong_type3_clone() {
        let left = normalize_tokens(
            "typescript",
            r#"export function alpha(input: number): number {
  let total = input
  total += 1
  total += 2
  total += 3
  total += 4
  total += 5
  total += 6
  total += 7
  total += 8
  total += 9
  total += 10
  total += 11
  total += 12
  return total
}"#,
        );
        let right = normalize_tokens(
            "typescript",
            r#"export function beta(input: number): number {
  let total = input
  total += 1
  total += 2
  total -= 3
  total += 4
  total += 5
  total += 6
  total -= 7
  total += 8
  total += 9
  total += 10
  total -= 11
  total += 12
  return total
}"#,
        );

        let matched = approximate_match_from_anchor(&left, 0, &right, 0, 8)
            .expect("strong type-3 clone should match");
        assert!(matched.common_tokens >= 32);
        assert!(matched.similarity_percent < 100);
    }

    fn token_stream(values: &[&str]) -> Vec<NormalizedToken> {
        values
            .iter()
            .enumerate()
            .map(|(idx, value)| NormalizedToken {
                value: (*value).to_string(),
                line: idx + 1,
            })
            .collect()
    }
}
