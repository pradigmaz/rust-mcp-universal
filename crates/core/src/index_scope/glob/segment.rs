use std::collections::HashMap;

use anyhow::Result;

use super::super::{MAX_MATCH_STATE_CELLS, MAX_MATCH_TEXT_CHARS};
use super::segment_eval::collect_positions;

mod match_helpers;
mod parser;

use self::match_helpers::segment_token_cost;
use self::parser::parse_segment_tokens_with_depth;

#[derive(Debug, Clone)]
pub(super) enum GroupKind {
    One,
    ZeroOrOne,
    OneOrMore,
    ZeroOrMore,
    Negated,
}

#[derive(Debug, Clone)]
pub(super) enum ClassItem {
    Single(char),
    Range(char, char),
}

#[derive(Debug, Clone)]
pub(super) struct CharClass {
    negated: bool,
    items: Vec<ClassItem>,
}

impl CharClass {
    pub(super) fn matches(&self, candidate: char) -> bool {
        let matched = self.items.iter().any(|item| match item {
            ClassItem::Single(ch) => *ch == candidate,
            ClassItem::Range(start, end) => *start <= candidate && candidate <= *end,
        });
        if self.negated { !matched } else { matched }
    }
}

#[derive(Debug, Clone)]
pub(super) enum SegmentToken {
    Literal(char),
    AnyChar,
    AnySeq,
    CharClass(CharClass),
    Group(GroupKind, Vec<Vec<SegmentToken>>),
}

pub(super) fn match_segment(pattern: &str, text: &str) -> bool {
    let Ok(tokens) = parse_segment_tokens(pattern) else {
        return false;
    };
    let chars = text.chars().collect::<Vec<_>>();
    if chars.len() > MAX_MATCH_TEXT_CHARS {
        return false;
    }
    let state_cells = segment_token_cost(&tokens).saturating_mul(chars.len().saturating_add(1));
    if state_cells > MAX_MATCH_STATE_CELLS {
        return false;
    }
    let mut memo = HashMap::new();
    collect_positions(&tokens, &chars, 0, 0, &mut memo).contains(&chars.len())
}

pub(super) fn parse_segment_tokens(pattern: &str) -> Result<Vec<SegmentToken>> {
    parse_segment_tokens_with_depth(pattern, 0)
}
