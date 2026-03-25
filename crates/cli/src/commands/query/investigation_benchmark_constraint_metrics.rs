use rmu_core::InvestigationConstraintLabel;
use serde_json::Value;

pub(super) fn constraint_matches_any<'a, I>(item: &Value, labels: I) -> bool
where
    I: IntoIterator<Item = &'a InvestigationConstraintLabel>,
{
    labels.into_iter().any(|label| {
        constraint_path(item) == Some(label.path.as_str())
            && label
                .constraint_kind
                .as_deref()
                .is_none_or(|kind| constraint_kind(item) == Some(kind))
            && label
                .source_kind
                .as_deref()
                .is_none_or(|kind| item["source_kind"].as_str() == Some(kind))
            && label
                .strength
                .as_deref()
                .is_none_or(|strength| item["strength"].as_str() == Some(strength))
    })
}

pub(super) fn constraint_path(item: &Value) -> Option<&str> {
    item["path"]
        .as_str()
        .or_else(|| item["source_path"].as_str())
}

pub(super) fn count_constraint(item: &Value, counts: &mut (usize, usize)) {
    count_field(constraint_kind(item), counts);
    count_field(item["source_kind"].as_str(), counts);
    count_field(constraint_path(item), counts);
    count_number(item["line_start"].as_u64(), counts);
    count_number(item["line_end"].as_u64(), counts);
    count_field(item["excerpt"].as_str(), counts);
    count_number(item["confidence"].as_f64(), counts);
    count_field(item["normalized_key"].as_str(), counts);
}

fn constraint_kind(item: &Value) -> Option<&str> {
    item["constraint_kind"]
        .as_str()
        .or_else(|| item["kind"].as_str())
}

fn count_field(value: Option<&str>, counts: &mut (usize, usize)) {
    counts.1 += 1;
    if value.is_some_and(|value| !value.trim().is_empty()) {
        counts.0 += 1;
    }
}

fn count_number<T>(value: Option<T>, counts: &mut (usize, usize)) {
    counts.1 += 1;
    if value.is_some() {
        counts.0 += 1;
    }
}
