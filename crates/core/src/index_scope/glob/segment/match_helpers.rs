use super::SegmentToken;

pub(super) fn segment_token_cost(tokens: &[SegmentToken]) -> usize {
    tokens
        .iter()
        .map(|token| match token {
            SegmentToken::Group(_, alternatives) => {
                1 + alternatives
                    .iter()
                    .map(|alt| segment_token_cost(alt))
                    .sum::<usize>()
            }
            _ => 1,
        })
        .sum()
}
