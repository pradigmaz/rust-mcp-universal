use anyhow::Result;

use super::super::Engine;

pub(super) fn load_top_languages_for_brief(
    engine: &Engine,
) -> Result<Vec<crate::model::WorkspaceLanguageStat>> {
    crate::engine_brief::load_top_languages_for_brief(engine, 8)
}

pub(super) fn load_top_symbols_for_brief(
    engine: &Engine,
) -> Result<Vec<crate::model::WorkspaceTopSymbol>> {
    crate::engine_brief::load_top_symbols_for_brief(engine, 12)
}

pub(super) fn make_brief_recommendations(status: &crate::model::IndexStatus) -> Vec<String> {
    crate::engine_brief::make_recommendations(status)
}
