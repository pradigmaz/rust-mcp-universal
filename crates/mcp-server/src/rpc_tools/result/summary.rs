use serde_json::Value;
pub(super) fn compact_content_summary(value: &Value) -> String {
    let Value::Object(object) = value else {
        return value.as_array().map_or_else(
            || "ok".to_string(),
            |items| format!("ok: items={}", items.len()),
        );
    };
    if object.contains_key("selected_context") {
        return report_summary(value);
    }
    if object.contains_key("brief") && object.contains_key("query_bundle") {
        return bootstrap_summary(value);
    }
    if object.contains_key("index_status") && object.contains_key("recommendations") {
        return brief_summary(value);
    }
    let parts = [
        "hits",
        "items",
        "buckets",
        "recent",
        "selected_context",
        "candidate_paths",
        "excluded_by_scope_paths",
        "warnings",
        "errors",
    ]
    .iter()
    .filter_map(|key| {
        object
            .get(*key)
            .and_then(Value::as_array)
            .map(|items| format!("{key}={}", items.len()))
    })
    .collect::<Vec<_>>();
    if parts.is_empty() {
        format!("ok: fields={}", object.len())
    } else {
        format!("ok: {}", parts.join(", "))
    }
}
fn brief_summary(value: &Value) -> String {
    let quality = value
        .pointer("/quality_summary/status")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let stale = u64_at(value, "/index_status/freshness/stale_files");
    let degradation =
        if let Some(reason) = value.pointer("/repair_hint/reason").and_then(Value::as_str) {
            reason.to_string()
        } else if stale > 0 {
            format!("stale_index:{stale}")
        } else if quality != "ready" && quality != "unknown" {
            format!("quality_summary:{quality}")
        } else {
            "none".to_string()
        };
    format!(
        "status={quality}; degradation_reason={degradation}; files={}; symbols={}",
        u64_at(value, "/index_status/files"),
        u64_at(value, "/index_status/symbols")
    )
}
fn report_summary(value: &Value) -> String {
    let hits = value.get("selected_context");
    format!(
        "status={}; degradation_reason={}; top_hits={}",
        hit_status(hits, "hits", "no_hits"),
        first_string(value.get("degradation_reasons")).unwrap_or("none"),
        top_hits(hits, true)
    )
}

fn bootstrap_summary(value: &Value) -> String {
    let hits = value.pointer("/query_bundle/hits");
    let context = value.pointer("/query_bundle/context/files");
    let top_hits = if has_items(context) {
        top_hits(context, true)
    } else {
        top_hits(hits, false)
    };
    format!(
        "status={}; degradation_reason={}; top_hits={}",
        hit_status(hits, "hits", "brief_only"),
        first_string(value.get("degradation_reasons")).unwrap_or("none"),
        top_hits
    )
}

fn hit_status(value: Option<&Value>, yes: &'static str, no: &'static str) -> &'static str {
    if has_items(value) { yes } else { no }
}

fn has_items(value: Option<&Value>) -> bool {
    value
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty())
}

fn top_hits(value: Option<&Value>, context_hit: bool) -> String {
    let Some(items) = value.and_then(Value::as_array) else {
        return "none".to_string();
    };
    let hits = items
        .iter()
        .take(3)
        .map(|item| hit_line(item, context_hit))
        .collect::<Vec<_>>();
    if hits.is_empty() {
        "none".to_string()
    } else {
        hits.join(" | ")
    }
}

fn hit_line(item: &Value, context_hit: bool) -> String {
    let path = item
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let score = item.get("score").and_then(Value::as_f64).unwrap_or(0.0);
    let reason = if context_hit {
        item.get("why")
            .and_then(Value::as_array)
            .and_then(|items| items.iter().find_map(Value::as_str))
            .unwrap_or("selected")
    } else {
        "search_hit"
    };
    let source = item
        .get(if context_hit {
            "chunk_source"
        } else {
            "language"
        })
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    format!("{path} ({reason}; score={score:.3}; source={source})")
}

fn first_string(value: Option<&Value>) -> Option<&str> {
    value
        .and_then(Value::as_array)
        .and_then(|items| items.iter().find_map(Value::as_str))
}

fn u64_at(value: &Value, pointer: &str) -> u64 {
    value.pointer(pointer).and_then(Value::as_u64).unwrap_or(0)
}
