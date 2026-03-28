use std::collections::{BTreeSet, HashMap};

use crate::model::QualityLocation;
use crate::quality::DuplicationFacts;
use crate::quality::duplication::artifact::{DuplicationCloneClass, DuplicationSignalRole};

use super::support::{LoadedCandidate, SegmentMember, merge_members, signal_token_floor};

pub(crate) fn build_file_facts(
    files: &[LoadedCandidate],
    clone_classes: &[DuplicationCloneClass],
) -> HashMap<String, DuplicationFacts> {
    let mut file_members = HashMap::<String, Vec<SegmentMember>>::new();
    let mut peers = HashMap::<String, BTreeSet<String>>::new();
    let mut similarity = HashMap::<String, i64>::new();
    let mut strongest_role = HashMap::<String, DuplicationSignalRole>::new();

    for class in clone_classes {
        if !class.cross_file || class.signal_role == DuplicationSignalRole::Boilerplate {
            continue;
        }
        let paths = class
            .members
            .iter()
            .map(|member| member.path.clone())
            .collect::<BTreeSet<_>>();
        for member in &class.members {
            file_members
                .entry(member.path.clone())
                .or_default()
                .push(SegmentMember {
                    path: member.path.clone(),
                    start_idx: 0,
                    end_idx: member.token_count,
                    start_line: member.start_line,
                    end_line: member.end_line,
                });
            similarity
                .entry(member.path.clone())
                .and_modify(|value| *value = (*value).max(class.similarity_percent))
                .or_insert(class.similarity_percent);
            strongest_role
                .entry(member.path.clone())
                .and_modify(|value| {
                    if role_rank(class.signal_role) < role_rank(*value) {
                        *value = class.signal_role;
                    }
                })
                .or_insert(class.signal_role);
            let entry = peers.entry(member.path.clone()).or_default();
            for peer in &paths {
                if peer != &member.path {
                    entry.insert(peer.clone());
                }
            }
        }
    }

    let mut out = HashMap::new();
    for file in files {
        let mut members = file_members.remove(&file.path).unwrap_or_default();
        members.sort_by(|left, right| {
            left.start_line
                .cmp(&right.start_line)
                .then_with(|| left.end_line.cmp(&right.end_line))
        });
        let merged = merge_members(&members);
        let duplicate_lines = merged
            .iter()
            .map(|member| member.end_line.saturating_sub(member.start_line) + 1)
            .sum::<usize>();
        let max_member = merged
            .iter()
            .max_by_key(|member| member.end_idx.saturating_sub(member.start_idx));
        let duplicate_density_bps = file
            .non_empty_lines
            .map(|lines| {
                ((duplicate_lines as i64).saturating_mul(10_000) / lines.max(1)).clamp(0, 10_000)
            })
            .unwrap_or_default();
        let max_duplicate_block_tokens = max_member
            .map(|member| {
                i64::try_from(member.end_idx.saturating_sub(member.start_idx)).unwrap_or(i64::MAX)
            })
            .unwrap_or_default();
        let role = strongest_role
            .remove(&file.path)
            .unwrap_or(DuplicationSignalRole::Primary);
        if max_duplicate_block_tokens < signal_token_floor(&file.path, &file.language, role) {
            continue;
        }
        out.insert(
            file.path.clone(),
            DuplicationFacts {
                duplicate_block_count: i64::try_from(merged.len()).unwrap_or(i64::MAX),
                duplicate_peer_count: i64::try_from(
                    peers.remove(&file.path).unwrap_or_default().len(),
                )
                .unwrap_or(i64::MAX),
                duplicate_lines: i64::try_from(duplicate_lines).unwrap_or(i64::MAX),
                max_duplicate_block_tokens,
                max_duplicate_similarity_percent: similarity.remove(&file.path).unwrap_or_default(),
                duplicate_density_bps,
                primary_location: max_member.map(|member| QualityLocation {
                    start_line: member.start_line,
                    start_column: 1,
                    end_line: member.end_line,
                    end_column: 1,
                }),
            },
        );
    }
    out
}

const fn role_rank(role: DuplicationSignalRole) -> u8 {
    match role {
        DuplicationSignalRole::Primary => 0,
        DuplicationSignalRole::Downweighted => 1,
        DuplicationSignalRole::Boilerplate => 2,
    }
}
