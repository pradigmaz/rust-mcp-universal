use std::collections::HashMap;

use super::segment::match_segment;

pub(super) fn glob_match_single_variant(pattern: &str, path: &str) -> bool {
    let pattern_segments = collapse_double_star_segments(split_segments(pattern));
    let path_segments = split_segments(path);
    let mut memo = HashMap::new();
    match_segments_from(&pattern_segments, 0, &path_segments, 0, &mut memo)
}

pub(super) fn split_segments(input: &str) -> Vec<&str> {
    input
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect()
}

fn collapse_double_star_segments(segments: Vec<&str>) -> Vec<&str> {
    let mut out = Vec::with_capacity(segments.len());
    for segment in segments {
        if segment == "**" && out.last().is_some_and(|prev| *prev == "**") {
            continue;
        }
        out.push(segment);
    }
    out
}

fn match_segments_from(
    pattern: &[&str],
    pi: usize,
    path: &[&str],
    si: usize,
    memo: &mut HashMap<(usize, usize), bool>,
) -> bool {
    if let Some(cached) = memo.get(&(pi, si)) {
        return *cached;
    }

    if pi == pattern.len() {
        let done = si == path.len();
        memo.insert((pi, si), done);
        return done;
    }

    let token = pattern[pi];
    let matched = if token == "**" {
        if pi + 1 == pattern.len() {
            true
        } else {
            match_segments_from(pattern, pi + 1, path, si, memo)
                || (si < path.len() && match_segments_from(pattern, pi, path, si + 1, memo))
        }
    } else if si >= path.len() || !match_segment(token, path[si]) {
        false
    } else {
        match_segments_from(pattern, pi + 1, path, si + 1, memo)
    };

    memo.insert((pi, si), matched);
    matched
}
