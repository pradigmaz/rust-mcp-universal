use time::{UtcOffset, format_description::well_known::Rfc3339};

use super::{
    ContextMode, IndexProfile, IndexingOptions, MigrationMode, PrivacyMode, RolloutPhase,
    SemanticFailMode,
};

#[test]
fn semantic_fail_mode_defaults_to_fail_open() {
    assert_eq!(SemanticFailMode::default(), SemanticFailMode::FailOpen);
}

#[test]
fn semantic_fail_mode_parse_rejects_unknown_values() {
    assert_eq!(
        SemanticFailMode::parse("fail_open"),
        Some(SemanticFailMode::FailOpen)
    );
    assert_eq!(
        SemanticFailMode::parse("fail_closed"),
        Some(SemanticFailMode::FailClosed)
    );
    assert_eq!(SemanticFailMode::parse("unknown"), None);
}

#[test]
fn privacy_mode_parse_rejects_unknown_values() {
    assert_eq!(PrivacyMode::parse("off"), Some(PrivacyMode::Off));
    assert_eq!(PrivacyMode::parse("mask"), Some(PrivacyMode::Mask));
    assert_eq!(PrivacyMode::parse("hash"), Some(PrivacyMode::Hash));
    assert_eq!(PrivacyMode::parse("unknown"), None);
}

#[test]
fn context_mode_parse_rejects_unknown_values() {
    assert_eq!(ContextMode::parse("code"), Some(ContextMode::Code));
    assert_eq!(ContextMode::parse("design"), Some(ContextMode::Design));
    assert_eq!(ContextMode::parse("bugfix"), Some(ContextMode::Bugfix));
    assert_eq!(ContextMode::parse("unknown"), None);
}

#[test]
fn rollout_phase_parse_rejects_unknown_values() {
    assert_eq!(RolloutPhase::parse("shadow"), Some(RolloutPhase::Shadow));
    assert_eq!(RolloutPhase::parse("canary_5"), Some(RolloutPhase::Canary5));
    assert_eq!(
        RolloutPhase::parse("canary_25"),
        Some(RolloutPhase::Canary25)
    );
    assert_eq!(RolloutPhase::parse("full_100"), Some(RolloutPhase::Full100));
    assert_eq!(RolloutPhase::parse("unknown"), None);
}

#[test]
fn migration_mode_parse_rejects_unknown_values() {
    assert_eq!(MigrationMode::parse("auto"), Some(MigrationMode::Auto));
    assert_eq!(MigrationMode::parse("off"), Some(MigrationMode::Off));
    assert_eq!(MigrationMode::parse("unknown"), None);
}

#[test]
fn index_profile_parse_rejects_unknown_values() {
    assert_eq!(
        IndexProfile::parse("rust-monorepo"),
        Some(IndexProfile::RustMonorepo)
    );
    assert_eq!(IndexProfile::parse("mixed"), Some(IndexProfile::Mixed));
    assert_eq!(
        IndexProfile::parse("docs-heavy"),
        Some(IndexProfile::DocsHeavy)
    );
    assert_eq!(IndexProfile::parse("unknown"), None);
}

#[test]
fn index_profile_serde_uses_kebab_case() {
    let raw = serde_json::to_string(&IndexProfile::DocsHeavy).expect("serialize profile");
    assert_eq!(raw, "\"docs-heavy\"");
    let parsed: IndexProfile = serde_json::from_str("\"rust-monorepo\"").expect("deserialize");
    assert_eq!(parsed, IndexProfile::RustMonorepo);
}

#[test]
fn index_profile_scope_resolver_returns_expected_rules() {
    assert_eq!(
        IndexProfile::RustMonorepo.include_paths(),
        &[
            "Cargo.toml",
            "Cargo.lock",
            "rust-toolchain",
            "rust-toolchain.toml",
            ".cargo",
            "crates",
            "src",
            "tests",
            "examples",
            "benches",
            "apps/**/Cargo.toml",
            "apps/**/src/**",
            "tools/**/Cargo.toml",
            "tools/**/src/**",
        ]
    );
    assert!(IndexProfile::Mixed.exclude_paths().contains(&"**/.next/**"));
    assert!(IndexProfile::DocsHeavy.include_paths().contains(&"**/*.md"));
    assert!(
        IndexProfile::DocsHeavy
            .exclude_paths()
            .contains(&"crates/**")
    );
}

#[test]
fn changed_since_parse_requires_rfc3339_with_timezone() {
    let parsed: IndexingOptions =
        serde_json::from_str(r#"{"changed_since":"2026-03-15T10:00:00+03:00"}"#)
            .expect("deserialize changed_since");
    let changed_since = parsed.changed_since.expect("changed_since present");
    assert_eq!(
        changed_since
            .to_offset(UtcOffset::UTC)
            .format(&Rfc3339)
            .expect("format changed_since"),
        "2026-03-15T07:00:00Z"
    );

    serde_json::from_str::<IndexingOptions>(r#"{"changed_since":"2026-03-15T10:00:00"}"#)
        .expect_err("timezone-less changed_since must fail");
}

#[test]
fn changed_since_serde_serializes_in_utc() {
    let parsed: IndexingOptions =
        serde_json::from_str(r#"{"changed_since":"2026-03-15T10:00:00+03:00"}"#)
            .expect("deserialize changed_since");
    let raw = serde_json::to_string(&parsed).expect("serialize changed_since");
    assert!(raw.contains("\"changed_since\":\"2026-03-15T07:00:00Z\""));
}
