use std::collections::{HashMap, VecDeque};

use crate::model::SearchHit;

use super::{is_mod_foundational_hit, module_root_bucket};

pub(super) fn rebalance_mod_runtime_prefix(hits: &mut Vec<SearchHit>, scan_limit: usize) {
    let scan_limit = hits.len().min(scan_limit.max(6));
    if scan_limit < 4 || distinct_module_roots(hits.iter().take(scan_limit)) < 2 {
        return;
    }

    let mut reordered = std::mem::take(hits);
    let mut tail = reordered.split_off(scan_limit);
    let prefix = reordered;

    let mut root_order = Vec::<String>::new();
    let mut groups = HashMap::<String, ModGroup>::new();
    let mut non_module = Vec::<SearchHit>::new();

    for hit in prefix {
        let Some(root) = module_root_bucket(&hit.path) else {
            non_module.push(hit);
            continue;
        };
        if !groups.contains_key(&root) {
            root_order.push(root.clone());
            groups.insert(root.clone(), ModGroup::default());
        }
        let group = groups
            .get_mut(&root)
            .expect("module root group should exist");
        if is_mod_foundational_hit(&hit) {
            group.foundational.push_back(hit);
        } else {
            group.secondary.push_back(hit);
        }
    }

    if root_order.len() < 2 {
        for root in root_order {
            if let Some(mut group) = groups.remove(&root) {
                non_module.extend(group.foundational.drain(..));
                non_module.extend(group.secondary.drain(..));
            }
        }
        non_module.append(&mut tail);
        *hits = non_module;
        return;
    }

    let mut balanced = Vec::<SearchHit>::with_capacity(scan_limit + tail.len());
    for root in &root_order {
        if let Some(group) = groups.get_mut(root)
            && let Some(hit) = group.foundational.pop_front()
        {
            balanced.push(hit);
        }
    }

    loop {
        let mut made_progress = false;
        for root in &root_order {
            let Some(group) = groups.get_mut(root) else {
                continue;
            };
            let next = group
                .foundational
                .pop_front()
                .or_else(|| group.secondary.pop_front());
            if let Some(hit) = next {
                balanced.push(hit);
                made_progress = true;
            }
        }
        if !made_progress {
            break;
        }
    }

    balanced.extend(non_module);
    balanced.append(&mut tail);
    *hits = balanced;
}

fn distinct_module_roots<'a>(hits: impl IntoIterator<Item = &'a SearchHit>) -> usize {
    let mut seen = Vec::<String>::new();
    for hit in hits {
        let Some(root) = module_root_bucket(&hit.path) else {
            continue;
        };
        if !seen.iter().any(|existing| existing == &root) {
            seen.push(root);
        }
    }
    seen.len()
}

#[derive(Default)]
struct ModGroup {
    foundational: VecDeque<SearchHit>,
    secondary: VecDeque<SearchHit>,
}
