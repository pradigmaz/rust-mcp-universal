use super::*;

pub(crate) fn run_symbol_lookup(
    engine: &Engine,
    json: bool,
    name: String,
    limit: usize,
    auto_index: bool,
    privacy_mode: PrivacyMode,
) -> Result<()> {
    let limit = require_min("limit", limit, 1)?;
    if name.trim().is_empty() {
        return Err(anyhow!("`name` must be non-empty"));
    }
    ensure_query_index_ready(engine, auto_index)?;
    let matches = engine.symbol_lookup(&name, limit)?;

    if json {
        let mut payload = serde_json::to_value(&matches)?;
        sanitize_value_for_privacy(privacy_mode, &mut payload);
        print_json(serde_json::to_string_pretty(&payload))?;
    } else {
        for symbol in matches {
            let exact = if symbol.exact { " exact" } else { "" };
            let location = render_location_suffix(symbol.line, symbol.column);
            print_line(format!(
                "[{kind}{exact}] {path}{location} :: {name}",
                kind = symbol.kind,
                path = sanitize_path_text(privacy_mode, &symbol.path),
                location = location,
                name = symbol.name,
            ));
        }
    }

    Ok(())
}

pub(crate) fn run_symbol_references(
    engine: &Engine,
    json: bool,
    name: String,
    limit: usize,
    auto_index: bool,
    privacy_mode: PrivacyMode,
) -> Result<()> {
    let limit = require_min("limit", limit, 1)?;
    if name.trim().is_empty() {
        return Err(anyhow!("`name` must be non-empty"));
    }
    ensure_query_index_ready(engine, auto_index)?;
    let hits = engine.symbol_references(&name, limit)?;

    if json {
        let mut payload = serde_json::to_value(&hits)?;
        sanitize_value_for_privacy(privacy_mode, &mut payload);
        print_json(serde_json::to_string_pretty(&payload))?;
    } else {
        for hit in hits {
            let exact = if hit.exact { " exact" } else { "" };
            let location = render_location_suffix(hit.line, hit.column);
            print_line(format!(
                "[refs={refs}{exact}] {path}{location} (lang={lang})",
                refs = hit.ref_count,
                path = sanitize_path_text(privacy_mode, &hit.path),
                location = location,
                lang = hit.language,
            ));
        }
    }

    Ok(())
}

fn render_location_suffix(line: Option<usize>, column: Option<usize>) -> String {
    match (line, column) {
        (Some(line), Some(column)) => format!(":{line}:{column}"),
        (Some(line), None) => format!(":{line}"),
        (None, _) => String::new(),
    }
}

pub(crate) fn run_related_files(
    engine: &Engine,
    json: bool,
    path: String,
    limit: usize,
    auto_index: bool,
    privacy_mode: PrivacyMode,
) -> Result<()> {
    let limit = require_min("limit", limit, 1)?;
    if path.trim().is_empty() {
        return Err(anyhow!("`path` must be non-empty"));
    }
    let _ = engine.ensure_mixed_index_ready_for_paths(auto_index, std::slice::from_ref(&path))?;
    let hits = engine.related_files(&path, limit)?;

    if json {
        let mut payload = serde_json::to_value(&hits)?;
        sanitize_value_for_privacy(privacy_mode, &mut payload);
        print_json(serde_json::to_string_pretty(&payload))?;
    } else {
        for hit in hits {
            print_line(format!(
                "[{score:.2}] {path} (lang={lang}, refs={refs}, deps={deps}, symbols={symbols})",
                score = hit.score,
                path = sanitize_path_text(privacy_mode, &hit.path),
                lang = hit.language,
                refs = hit.ref_overlap,
                deps = hit.dep_overlap,
                symbols = hit.symbol_overlap,
            ));
        }
    }

    Ok(())
}

pub(crate) fn run_call_path(
    engine: &Engine,
    json: bool,
    from: String,
    to: String,
    max_hops: usize,
    auto_index: bool,
    privacy_mode: PrivacyMode,
) -> Result<()> {
    let max_hops = require_min("max_hops", max_hops, 1)?;
    if from.trim().is_empty() {
        return Err(anyhow!("`from` must be non-empty"));
    }
    if to.trim().is_empty() {
        return Err(anyhow!("`to` must be non-empty"));
    }
    ensure_query_index_ready(engine, auto_index)?;
    let result = engine.call_path(&from, &to, max_hops)?;

    if json {
        let mut payload = serde_json::to_value(&result)?;
        sanitize_value_for_privacy(privacy_mode, &mut payload);
        print_json(serde_json::to_string_pretty(&payload))?;
    } else if result.found {
        print_line(format!(
            "found=true hops={} total_weight={:.2} from={} to={}",
            result.hops,
            result.total_weight,
            sanitize_path_text(privacy_mode, &result.from.resolved_path),
            sanitize_path_text(privacy_mode, &result.to.resolved_path),
        ));
        for step in result.steps {
            let location = render_location_suffix(step.line, step.column);
            print_line(format!(
                "{} --{}[{:.2}, raw_count={}, evidence={}{}]--> {}",
                sanitize_path_text(privacy_mode, &step.from_path),
                step.edge_kind,
                step.weight,
                step.raw_count,
                step.evidence,
                location,
                sanitize_path_text(privacy_mode, &step.to_path),
            ));
        }
    } else {
        print_line(format!(
            "found=false from={} to={} visited_nodes={} considered_edges={} max_hops={}",
            sanitize_path_text(privacy_mode, &result.from.resolved_path),
            sanitize_path_text(privacy_mode, &result.to.resolved_path),
            result.explain.visited_nodes,
            result.explain.considered_edges,
            result.explain.max_hops,
        ));
    }

    Ok(())
}
