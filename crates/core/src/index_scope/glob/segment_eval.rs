use std::collections::{HashMap, HashSet, VecDeque};

use super::segment::{GroupKind, SegmentToken};

pub(super) fn collect_positions(
    tokens: &[SegmentToken],
    text: &[char],
    ti: usize,
    si: usize,
    memo: &mut HashMap<(usize, usize), Vec<usize>>,
) -> Vec<usize> {
    if let Some(cached) = memo.get(&(ti, si)) {
        return cached.clone();
    }

    let result = if ti == tokens.len() {
        vec![si]
    } else {
        match &tokens[ti] {
            SegmentToken::Literal(expected) => {
                if si < text.len() && text[si] == *expected {
                    collect_positions(tokens, text, ti + 1, si + 1, memo)
                } else {
                    Vec::new()
                }
            }
            SegmentToken::AnyChar => {
                if si < text.len() {
                    collect_positions(tokens, text, ti + 1, si + 1, memo)
                } else {
                    Vec::new()
                }
            }
            SegmentToken::AnySeq => {
                let mut out = Vec::new();
                for next in si..=text.len() {
                    out.extend(collect_positions(tokens, text, ti + 1, next, memo));
                }
                dedup_positions(out)
            }
            SegmentToken::CharClass(class) => {
                if si < text.len() && class.matches(text[si]) {
                    collect_positions(tokens, text, ti + 1, si + 1, memo)
                } else {
                    Vec::new()
                }
            }
            SegmentToken::Group(kind, alternatives) => {
                let mut out = Vec::new();
                match kind {
                    GroupKind::One => {
                        for end in one_group_end_positions(alternatives, text, si) {
                            out.extend(collect_positions(tokens, text, ti + 1, end, memo));
                        }
                    }
                    GroupKind::ZeroOrOne => {
                        out.extend(collect_positions(tokens, text, ti + 1, si, memo));
                        for end in one_group_end_positions(alternatives, text, si) {
                            out.extend(collect_positions(tokens, text, ti + 1, end, memo));
                        }
                    }
                    GroupKind::ZeroOrMore => {
                        for end in repeated_group_end_positions(alternatives, text, si, false) {
                            out.extend(collect_positions(tokens, text, ti + 1, end, memo));
                        }
                    }
                    GroupKind::OneOrMore => {
                        for end in repeated_group_end_positions(alternatives, text, si, true) {
                            out.extend(collect_positions(tokens, text, ti + 1, end, memo));
                        }
                    }
                    GroupKind::Negated => {
                        let mut disallowed = HashSet::new();
                        for alt in alternatives {
                            for end in collect_alt_positions(alt, text, si) {
                                disallowed.insert(end);
                            }
                        }
                        for end in si..=text.len() {
                            if disallowed.contains(&end) {
                                continue;
                            }
                            out.extend(collect_positions(tokens, text, ti + 1, end, memo));
                        }
                    }
                }
                dedup_positions(out)
            }
        }
    };

    memo.insert((ti, si), result.clone());
    result
}

fn one_group_end_positions(
    alternatives: &[Vec<SegmentToken>],
    text: &[char],
    start: usize,
) -> Vec<usize> {
    let mut out = Vec::new();
    let mut alt_cache: HashMap<(usize, usize), Vec<usize>> = HashMap::new();
    for (alt_idx, alt) in alternatives.iter().enumerate() {
        out.extend(collect_alt_positions_cached(
            alt,
            alt_idx,
            text,
            start,
            &mut alt_cache,
        ));
    }
    dedup_positions(out)
}

fn repeated_group_end_positions(
    alternatives: &[Vec<SegmentToken>],
    text: &[char],
    start: usize,
    require_at_least_one: bool,
) -> Vec<usize> {
    let mut queue = VecDeque::new();
    let mut seen = HashSet::new();
    let mut out = HashSet::new();
    let mut alt_cache: HashMap<(usize, usize), Vec<usize>> = HashMap::new();

    queue.push_back(start);
    seen.insert(start);
    if !require_at_least_one {
        out.insert(start);
    }

    while let Some(pos) = queue.pop_front() {
        for (alt_idx, alt) in alternatives.iter().enumerate() {
            for end in collect_alt_positions_cached(alt, alt_idx, text, pos, &mut alt_cache) {
                if end == pos {
                    continue;
                }
                out.insert(end);
                if seen.insert(end) {
                    queue.push_back(end);
                }
            }
        }
    }

    let mut values = out.into_iter().collect::<Vec<_>>();
    values.sort_unstable();
    values
}

fn collect_alt_positions(tokens: &[SegmentToken], text: &[char], start: usize) -> Vec<usize> {
    let mut memo = HashMap::new();
    collect_positions(tokens, text, 0, start, &mut memo)
}

fn collect_alt_positions_cached(
    tokens: &[SegmentToken],
    alt_idx: usize,
    text: &[char],
    start: usize,
    cache: &mut HashMap<(usize, usize), Vec<usize>>,
) -> Vec<usize> {
    if let Some(cached) = cache.get(&(alt_idx, start)) {
        return cached.clone();
    }
    let positions = collect_alt_positions(tokens, text, start);
    cache.insert((alt_idx, start), positions.clone());
    positions
}

fn dedup_positions(mut values: Vec<usize>) -> Vec<usize> {
    values.sort_unstable();
    values.dedup();
    values
}
