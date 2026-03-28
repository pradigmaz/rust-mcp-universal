use std::collections::BTreeMap;

use super::artifact::DuplicationCloneClass;

pub(crate) fn collapse_nested_clone_classes(
    mut clone_classes: Vec<DuplicationCloneClass>,
) -> Vec<DuplicationCloneClass> {
    clone_classes.sort_by(|left, right| {
        right
            .normalized_token_count
            .cmp(&left.normalized_token_count)
            .then_with(|| right.similarity_percent.cmp(&left.similarity_percent))
            .then_with(|| left.clone_class_id.cmp(&right.clone_class_id))
    });
    let mut kept = Vec::<DuplicationCloneClass>::new();
    'candidate: for class in clone_classes {
        for existing in &kept {
            if classes_overlap(existing, &class) {
                continue 'candidate;
            }
        }
        kept.push(class);
    }
    kept.sort_by(|left, right| left.clone_class_id.cmp(&right.clone_class_id));
    kept
}

fn classes_overlap(wider: &DuplicationCloneClass, narrower: &DuplicationCloneClass) -> bool {
    if wider.language != narrower.language || wider.cross_file != narrower.cross_file {
        return false;
    }
    let wider_spans = member_spans(wider);
    let narrower_spans = member_spans(narrower);
    wider_spans.len() == narrower_spans.len()
        && !wider_spans.is_empty()
        && narrower_spans.into_iter().all(|(path, (start, end))| {
            wider_spans
                .get(&path)
                .is_some_and(|(outer_start, outer_end)| start >= *outer_start && end <= *outer_end)
        })
}

fn member_spans(class: &DuplicationCloneClass) -> BTreeMap<String, (usize, usize)> {
    let mut spans = BTreeMap::<String, (usize, usize)>::new();
    for member in &class.members {
        spans
            .entry(member.path.clone())
            .and_modify(|span| {
                span.0 = span.0.min(member.start_line);
                span.1 = span.1.max(member.end_line);
            })
            .or_insert((member.start_line, member.end_line));
    }
    spans
}
