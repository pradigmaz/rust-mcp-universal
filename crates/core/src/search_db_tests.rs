use super::{
    compare_hits_desc, escape_like_value, extract_tokens, graph_boost, keep_top_hits,
    like_prefilter_limit, like_scan_budget, like_score, path_match_boost, search_like,
};
use crate::model::SearchHit;
use crate::query_profile::QueryProfile;
use rusqlite::{Connection, params};

fn setup_files_table(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(
        "CREATE TABLE files (path TEXT PRIMARY KEY, sample TEXT NOT NULL, size_bytes INTEGER NOT NULL, language TEXT NOT NULL);",
    )?;
    Ok(())
}

fn setup_graph_tables(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(
        "CREATE TABLE symbols (path TEXT NOT NULL, name TEXT NOT NULL, kind TEXT NOT NULL, language TEXT NOT NULL, line INTEGER, column INTEGER);
         CREATE TABLE refs (path TEXT NOT NULL, symbol TEXT NOT NULL, language TEXT NOT NULL, line INTEGER, column INTEGER);
         CREATE TABLE module_deps (path TEXT NOT NULL, dep TEXT NOT NULL, language TEXT NOT NULL);",
    )?;
    Ok(())
}

fn hit(path: &str, score: f32) -> SearchHit {
    SearchHit {
        path: path.to_string(),
        preview: String::new(),
        score,
        size_bytes: 1,
        language: "rust".to_string(),
    }
}

#[test]
fn unicode_tokenization_and_lowercase() {
    let tokens = extract_tokens("Straße ПРИВЕТ_Мир 東京");
    assert!(tokens.contains(&"straße".to_string()));
    assert!(tokens.contains(&"привет_мир".to_string()));
    assert!(tokens.contains(&"東京".to_string()));
}

#[test]
fn like_wildcards_are_escaped() {
    assert_eq!(escape_like_value(r"a%b_c\z"), r"a\%b\_c\\z");
}

#[test]
fn fallback_matches_unicode_case_insensitively() -> anyhow::Result<()> {
    let conn = Connection::open_in_memory()?;
    setup_files_table(&conn)?;
    conn.execute(
        "INSERT INTO files(path, sample, size_bytes, language) VALUES (?1, ?2, ?3, ?4)",
        params!["src/hello.rs", "привет мир", 10_i64, "rust"],
    )?;

    let hits = search_like(&conn, "ПРИВЕТ", 10)?;
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].path, "src/hello.rs");
    Ok(())
}

#[test]
fn fallback_regression_does_not_rely_on_sqlite_lower_for_unicode() -> anyhow::Result<()> {
    let conn = Connection::open_in_memory()?;
    setup_files_table(&conn)?;
    conn.execute(
        "INSERT INTO files(path, sample, size_bytes, language) VALUES (?1, ?2, ?3, ?4)",
        params!["src/turkish.rs", "İSTANBUL ve ПРИВЕТ", 20_i64, "rust"],
    )?;

    let turkish_hits = search_like(&conn, "istanbul", 10)?;
    assert!(turkish_hits.iter().any(|hit| hit.path == "src/turkish.rs"));

    let cyrillic_hits = search_like(&conn, "привет", 10)?;
    assert!(cyrillic_hits.iter().any(|hit| hit.path == "src/turkish.rs"));
    Ok(())
}

#[test]
fn fallback_matches_canonical_equivalent_unicode_forms() -> anyhow::Result<()> {
    let conn = Connection::open_in_memory()?;
    setup_files_table(&conn)?;
    conn.execute(
        "INSERT INTO files(path, sample, size_bytes, language) VALUES (?1, ?2, ?3, ?4)",
        params!["src/unicode.rs", "Cafe\u{301} menu", 12_i64, "rust"],
    )?;

    let hits = search_like(&conn, "CAFÉ", 10)?;
    assert!(hits.iter().any(|hit| hit.path == "src/unicode.rs"));
    Ok(())
}

#[test]
fn position_bonus_uses_char_offsets_not_byte_offsets() {
    let score = like_score("п", "", "😀😀п");
    let expected = 0.25 + 0.07 + (0.12 / 3.0);
    assert!((score - expected).abs() < 1e-6);
}

#[test]
fn keep_top_hits_overflow_keeps_best_and_tie_breaks_by_path() {
    let mut best_hits = Vec::new();
    keep_top_hits(&mut best_hits, hit("src/c.rs", 1.0), 2);
    keep_top_hits(&mut best_hits, hit("src/b.rs", 1.0), 2);

    keep_top_hits(&mut best_hits, hit("src/a.rs", 1.0), 2);
    keep_top_hits(&mut best_hits, hit("src/z.rs", 0.2), 2);

    best_hits.sort_by(compare_hits_desc);
    assert_eq!(best_hits.len(), 2);
    assert_eq!(best_hits[0].path, "src/a.rs");
    assert_eq!(best_hits[1].path, "src/b.rs");
}

#[test]
fn graph_boost_surfaces_graph_query_errors_instead_of_silent_zero() -> anyhow::Result<()> {
    let conn = Connection::open_in_memory()?;
    let tokens = vec!["needle".to_string()];

    let err = graph_boost(&conn, "src/lib.rs", &tokens, QueryProfile::Precise)
        .expect_err("missing graph tables must propagate as errors");
    assert!(err.to_string().contains("failed to query symbols boost"));
    Ok(())
}

#[test]
fn graph_boost_scales_down_for_broad_queries_without_touching_precise_profile() -> anyhow::Result<()>
{
    let conn = Connection::open_in_memory()?;
    setup_graph_tables(&conn)?;
    conn.execute(
        "INSERT INTO symbols(path, name, kind, language, line, column) VALUES (?1, ?2, ?3, ?4, NULL, NULL)",
        params!["src/needle.rs", "needle", "function", "rust"],
    )?;
    conn.execute(
        "INSERT INTO refs(path, symbol, language, line, column) VALUES (?1, ?2, ?3, NULL, NULL)",
        params!["src/needle.rs", "needle", "rust"],
    )?;
    conn.execute(
        "INSERT INTO module_deps(path, dep, language) VALUES (?1, ?2, ?3)",
        params!["src/needle.rs", "needle", "rust"],
    )?;

    let tokens = vec!["needle".to_string()];
    let precise = graph_boost(&conn, "src/needle.rs", &tokens, QueryProfile::Precise)?;
    let balanced = graph_boost(&conn, "src/needle.rs", &tokens, QueryProfile::Balanced)?;
    let bugfix = graph_boost(&conn, "src/needle.rs", &tokens, QueryProfile::Bugfix)?;
    let exploratory = graph_boost(&conn, "src/needle.rs", &tokens, QueryProfile::Exploratory)?;

    assert!((precise - 0.294).abs() < 1e-6);
    assert!((balanced - 0.3675).abs() < 1e-6);
    assert!((bugfix - 0.343).abs() < 1e-6);
    assert!((exploratory - 0.2695).abs() < 1e-6);
    Ok(())
}

#[test]
fn path_match_boost_prefers_exact_file_stem_over_generic_container() {
    let tokens = vec!["symbol_lookup".to_string()];
    let exact = path_match_boost("src/rpc_tools/handlers/symbol_lookup.rs", &tokens);
    let umbrella = path_match_boost("src/rpc_tools/handlers.rs", &tokens);

    assert!(exact > umbrella);
    assert!(exact >= 0.85);
    assert_eq!(umbrella, 0.0);
}

#[test]
fn like_prefilter_limit_is_clamped() {
    assert_eq!(like_prefilter_limit(1), 256);
    assert_eq!(like_prefilter_limit(10), 960);
    assert_eq!(like_prefilter_limit(10_000), 8_192);
}

#[test]
fn like_scan_budget_is_clamped() {
    assert_eq!(like_scan_budget(1), 2_048);
    assert_eq!(like_scan_budget(10), 3_840);
    assert_eq!(like_scan_budget(10_000), 65_536);
}
