use crate::quality::duplication::artifact::DuplicationSignalRole;
use crate::quality::duplication::classify::QualityCorpusClass;
use crate::quality::duplication::surface::signal_token_floor_for_surface;
use crate::quality::duplication::tokenize::NormalizedToken;
use crate::utils::hash_bytes;

#[derive(Debug, Clone)]
pub(crate) struct LoadedCandidate {
    pub(crate) path: String,
    pub(crate) language: String,
    pub(crate) corpus_class: QualityCorpusClass,
    pub(crate) tokens: Vec<NormalizedToken>,
    pub(crate) non_empty_lines: Option<i64>,
}

#[derive(Debug, Clone)]
pub(crate) struct SegmentMember {
    pub(crate) path: String,
    pub(crate) start_idx: usize,
    pub(crate) end_idx: usize,
    pub(crate) start_line: usize,
    pub(crate) end_line: usize,
}

pub(crate) fn class_id_for_signature(
    language: &str,
    corpus_class: QualityCorpusClass,
    signature_tokens: &[String],
) -> String {
    let joined = signature_tokens.join("\u{1f}");
    hash_bytes(format!("{language}\u{1f}{}\u{1f}{joined}", corpus_class.as_str()).as_bytes())
}

pub(crate) fn build_member_span(
    path: &str,
    tokens: &[NormalizedToken],
    start_idx: usize,
    end_idx: usize,
) -> SegmentMember {
    SegmentMember {
        path: path.to_string(),
        start_idx,
        end_idx,
        start_line: tokens[start_idx].line,
        end_line: tokens[end_idx - 1].line,
    }
}

pub(crate) fn member_key(member: &SegmentMember) -> String {
    format!("{}:{}:{}", member.path, member.start_line, member.end_line)
}

pub(crate) fn merge_members(members: &[SegmentMember]) -> Vec<SegmentMember> {
    let mut merged = Vec::<SegmentMember>::new();
    for member in members {
        match merged.last_mut() {
            Some(current)
                if current.path == member.path && member.start_line <= current.end_line + 1 =>
            {
                current.end_line = current.end_line.max(member.end_line);
                current.end_idx = current.end_idx.max(member.end_idx);
            }
            _ => merged.push(member.clone()),
        }
    }
    merged
}

pub(crate) fn ranges_overlap(
    left_start: usize,
    left_end: usize,
    right_start: usize,
    right_end: usize,
) -> bool {
    left_start < right_end && right_start < left_end
}

pub(crate) fn extend_match(
    left: &[NormalizedToken],
    left_start: usize,
    right: &[NormalizedToken],
    right_start: usize,
) -> (usize, usize, usize) {
    let mut left_idx = left_start;
    let mut right_idx = right_start;
    while left_idx > 0 && right_idx > 0 && left[left_idx - 1].value == right[right_idx - 1].value {
        left_idx -= 1;
        right_idx -= 1;
    }
    let mut length = 0;
    while left_idx + length < left.len()
        && right_idx + length < right.len()
        && left[left_idx + length].value == right[right_idx + length].value
    {
        length += 1;
    }
    (left_idx, right_idx, length)
}

pub(crate) fn window_hash(tokens: &[NormalizedToken], start: usize, len: usize) -> String {
    let joined = tokens[start..start + len]
        .iter()
        .map(|token| token.value.as_str())
        .collect::<Vec<_>>()
        .join("\u{1f}");
    hash_bytes(joined.as_bytes())
}

pub(crate) fn signal_token_floor(path: &str, language: &str, role: DuplicationSignalRole) -> i64 {
    signal_token_floor_for_surface(path, language, role)
}
