#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum QualityCorpusClass {
    Production,
    Test,
    Config,
    Generated,
    Prototype,
    Backup,
    Resource,
    ArtifactCache,
}

impl QualityCorpusClass {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Production => "production",
            Self::Test => "test",
            Self::Config => "config",
            Self::Generated => "generated",
            Self::Prototype => "prototype",
            Self::Backup => "backup",
            Self::Resource => "resource",
            Self::ArtifactCache => "artifact_cache",
        }
    }

    pub(crate) const fn participates_in_duplication(self) -> bool {
        matches!(self, Self::Production)
    }
}

pub(crate) fn classify_corpus(rel_path: &str, language: &str) -> QualityCorpusClass {
    let normalized = rel_path.replace('\\', "/");
    let lowered = normalized.to_ascii_lowercase();
    let segments = lowered.split('/').collect::<Vec<_>>();
    if lowered.starts_with(".rmu/")
        || lowered.starts_with("target/")
        || lowered.starts_with("dist/")
        || lowered.starts_with("build/")
        || lowered.starts_with(".gradle/")
        || lowered.starts_with("run/")
        || lowered.contains("/__pycache__/")
        || lowered.contains("/node_modules/")
        || lowered.contains("/coverage/")
        || lowered.contains("/.next/")
        || lowered.contains("/.turbo/")
    {
        return QualityCorpusClass::ArtifactCache;
    }
    if lowered.contains("/_legacy")
        || lowered.contains("/backup/")
        || lowered.contains("/backups/")
        || lowered.contains("/old/")
        || lowered.ends_with(".bak")
        || lowered.ends_with(".old")
    {
        return QualityCorpusClass::Backup;
    }
    if lowered.contains("/prototype")
        || lowered.contains("/prototypes/")
        || lowered.contains("/demo/")
        || lowered.contains("/draft/")
        || lowered.contains("/scratch/")
    {
        return QualityCorpusClass::Prototype;
    }
    if lowered.contains("/generated/")
        || lowered.contains("/gen/")
        || lowered.contains(".generated.")
        || lowered.contains(".gen.")
    {
        return QualityCorpusClass::Generated;
    }
    if lowered.contains("/migrations/") || lowered.contains("/migration/") {
        return QualityCorpusClass::Config;
    }
    if matches!(language, "json" | "css" | "html" | "yaml" | "yml" | "toml") {
        return QualityCorpusClass::Resource;
    }
    if lowered.starts_with("tests/")
        || lowered.contains("/tests/")
        || lowered.starts_with("benches/")
        || lowered.contains("/benches/")
        || lowered.contains(".test.")
        || lowered.contains(".spec.")
        || lowered.contains("_test.")
        || segments.iter().copied().any(is_test_segment)
    {
        return QualityCorpusClass::Test;
    }
    if matches!(language, "toml" | "json" | "yaml" | "yml" | "ini" | "cfg") {
        return QualityCorpusClass::Config;
    }
    QualityCorpusClass::Production
}

fn is_test_segment(segment: &str) -> bool {
    matches!(segment, "test" | "tests" | "bench" | "benches")
        || segment.starts_with("test_")
        || segment.starts_with("tests_")
        || segment.ends_with("_test")
        || segment.ends_with("_tests")
}

#[cfg(test)]
mod tests {
    use super::{QualityCorpusClass, classify_corpus};

    #[test]
    fn classifier_uses_only_universal_test_generated_and_config_conventions() {
        assert_eq!(
            classify_corpus(
                "src/world/generation/runtime/BiomeSurfaceRules.test.ts",
                "typescript",
            ),
            QualityCorpusClass::Test
        );
        assert_eq!(
            classify_corpus("crates/mcp-server/src/main_tests/protocol/tools_call.rs", "rust"),
            QualityCorpusClass::Test
        );
        assert_eq!(
            classify_corpus(
                "crates/mcp-server/src/rpc_tools_tests/tools/indexing_and_alias.rs",
                "rust",
            ),
            QualityCorpusClass::Test
        );
        assert_eq!(
            classify_corpus(
                "crates/core/src/engine/tests_quality/duplication_precision.rs",
                "rust",
            ),
            QualityCorpusClass::Test
        );
        assert_eq!(
            classify_corpus("db/migrations/001_init.sql", "sql"),
            QualityCorpusClass::Config
        );
        assert_eq!(
            classify_corpus("frontend/src/generated/client.gen.ts", "typescript"),
            QualityCorpusClass::Generated
        );
        assert_eq!(
            classify_corpus("config/app/settings.toml", "toml"),
            QualityCorpusClass::Resource
        );
    }

    #[test]
    fn classifier_does_not_encode_domain_or_framework_specific_paths() {
        assert_eq!(
            classify_corpus(
                "mods/kiron_client/src/client/java/org/kiron/kiron_client/mixin/client/MixinSodiumOcclusionCuller.java",
                "java",
            ),
            QualityCorpusClass::Production
        );
        assert_eq!(
            classify_corpus("api/routes/chapter_routes.py", "python"),
            QualityCorpusClass::Production
        );
        assert_eq!(
            classify_corpus("backend/app/api/v1/endpoints/admin_schedule.py", "python"),
            QualityCorpusClass::Production
        );
        assert_eq!(
            classify_corpus("src/types/Mobs.ts", "typescript"),
            QualityCorpusClass::Production
        );
        assert_eq!(
            classify_corpus("src/constants/GameConstants.ts", "typescript"),
            QualityCorpusClass::Production
        );
        assert_eq!(
            classify_corpus("frontend/src/components/ui/dropdown-menu.tsx", "typescript"),
            QualityCorpusClass::Production
        );
        assert_eq!(
            classify_corpus("backend/alembic/versions/065_autobalance_attestation.py", "python"),
            QualityCorpusClass::Production
        );
        assert_eq!(
            classify_corpus(
                "mods/veinmining/src/main/java/org/mine/veinmining/network/packet/VeinMiningStatePacket.java",
                "java",
            ),
            QualityCorpusClass::Production
        );
    }
}
