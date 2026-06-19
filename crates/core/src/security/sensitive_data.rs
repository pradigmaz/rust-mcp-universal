use std::fs;

use anyhow::Result;
use ignore::WalkBuilder;

use crate::engine::Engine;
use crate::model::{
    FindingConfidence, SensitiveDataOptions, SensitiveDataResult, SensitiveDataSummary,
    SignalMemoryStatus,
};

use super::sensitive_data_ordering::compare_findings;
use super::sensitive_data_patterns::{matches_path_prefix, scan_file, skip_path};

const MAX_SCAN_BYTES: u64 = 262_144;

pub(super) fn scan_sensitive_data(
    engine: &Engine,
    options: &SensitiveDataOptions,
) -> Result<SensitiveDataResult> {
    let memory = crate::signal_memory::load_signal_memory(&engine.project_root)?;
    let mut evaluated_files = 0_usize;
    let mut hits = Vec::new();

    let walker = WalkBuilder::new(&engine.project_root)
        .hidden(false)
        .git_ignore(true)
        .git_exclude(true)
        .build();
    for entry in walker {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        if !entry.file_type().is_some_and(|kind| kind.is_file()) {
            continue;
        }
        let Ok(rel_path) = entry.path().strip_prefix(&engine.project_root) else {
            continue;
        };
        let rel_path = rel_path.to_string_lossy().replace('\\', "/");
        if !matches_path_prefix(&rel_path, options.path_prefix.as_deref()) || skip_path(&rel_path) {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if metadata.len() > MAX_SCAN_BYTES {
            continue;
        }
        let Ok(bytes) = fs::read(entry.path()) else {
            continue;
        };
        if bytes.contains(&0) {
            continue;
        }
        evaluated_files += 1;
        let text = String::from_utf8_lossy(&bytes);
        hits.extend(scan_file(
            &rel_path,
            &text,
            options.include_low_confidence,
            &memory,
        ));
    }

    hits.sort_by(compare_findings);
    let remembered_noisy_findings = hits
        .iter()
        .filter(|finding| finding.memory_status == Some(SignalMemoryStatus::RememberedNoisy))
        .count();
    let high_confidence_findings = hits
        .iter()
        .filter(|finding| finding.confidence == FindingConfidence::High)
        .count();
    hits.truncate(options.limit);

    Ok(SensitiveDataResult {
        summary: SensitiveDataSummary {
            evaluated_files,
            findings: hits.len(),
            high_confidence_findings,
            remembered_noisy_findings,
        },
        hits,
    })
}
