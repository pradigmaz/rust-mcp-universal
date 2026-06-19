use std::cmp::Ordering;

use crate::model::{
    FindingConfidence, SensitiveDataFinding, SensitiveDataRotationUrgency, SignalMemoryStatus,
};

pub(super) fn compare_findings(
    left: &SensitiveDataFinding,
    right: &SensitiveDataFinding,
) -> Ordering {
    confidence_rank(right.confidence)
        .cmp(&confidence_rank(left.confidence))
        .then_with(|| {
            urgency_rank(right.rotation_urgency).cmp(&urgency_rank(left.rotation_urgency))
        })
        .then_with(|| noisy_rank(left).cmp(&noisy_rank(right)))
        .then_with(|| left.path.cmp(&right.path))
        .then_with(|| {
            left.location
                .as_ref()
                .map(|location| location.start_line)
                .unwrap_or(usize::MAX)
                .cmp(
                    &right
                        .location
                        .as_ref()
                        .map(|location| location.start_line)
                        .unwrap_or(usize::MAX),
                )
        })
}

fn confidence_rank(confidence: FindingConfidence) -> usize {
    match confidence {
        FindingConfidence::High => 3,
        FindingConfidence::Medium => 2,
        FindingConfidence::Low => 1,
    }
}

fn urgency_rank(urgency: SensitiveDataRotationUrgency) -> usize {
    match urgency {
        SensitiveDataRotationUrgency::Critical => 3,
        SensitiveDataRotationUrgency::High => 2,
        SensitiveDataRotationUrgency::Medium => 1,
    }
}

fn noisy_rank(finding: &SensitiveDataFinding) -> usize {
    match (finding.confidence, finding.memory_status) {
        (FindingConfidence::High, _) => 0,
        (_, Some(SignalMemoryStatus::RememberedNoisy)) => 2,
        (_, Some(SignalMemoryStatus::RememberedUseful)) => 0,
        _ => 1,
    }
}
