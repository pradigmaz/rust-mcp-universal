#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QueryProfile {
    Precise,
    Balanced,
    Exploratory,
    Bugfix,
}

pub(crate) fn derive_query_profile(query: &str) -> QueryProfile {
    let query_lc = query.to_ascii_lowercase();
    let token_count = query
        .split(|ch: char| !ch.is_alphanumeric() && ch != '_')
        .filter(|token| !token.is_empty())
        .count();
    let symbol_punctuation = query.contains("::")
        || query.contains('/')
        || query.contains('\\')
        || query.contains('_')
        || query.contains('.');
    let symbol_like = query.contains("::") || (symbol_punctuation && token_count <= 2);
    let bugfix_like = [
        "bug",
        "error",
        "fail",
        "panic",
        "crash",
        "regression",
        "issue",
    ]
    .iter()
    .any(|marker| query_lc.contains(marker));

    if bugfix_like {
        QueryProfile::Bugfix
    } else if symbol_like || token_count <= 2 {
        QueryProfile::Precise
    } else if token_count >= 7 {
        QueryProfile::Exploratory
    } else {
        QueryProfile::Balanced
    }
}

pub(crate) fn graph_boost_scale(profile: QueryProfile) -> f32 {
    match profile {
        QueryProfile::Precise => 0.60,
        QueryProfile::Balanced => 0.75,
        QueryProfile::Exploratory => 0.55,
        QueryProfile::Bugfix => 0.70,
    }
}

#[cfg(test)]
mod tests {
    use super::{QueryProfile, derive_query_profile, graph_boost_scale};

    #[test]
    fn derive_query_profile_treats_symbol_queries_as_precise() {
        assert_eq!(
            derive_query_profile("symbol_references"),
            QueryProfile::Precise
        );
        assert_eq!(
            derive_query_profile("crate::engine::navigation"),
            QueryProfile::Precise
        );
    }

    #[test]
    fn derive_query_profile_treats_long_nl_queries_as_exploratory() {
        assert_eq!(
            derive_query_profile(
                "How are Rust symbol references extracted and linked to line and column positions in this project?"
            ),
            QueryProfile::Exploratory
        );
    }

    #[test]
    fn derive_query_profile_keeps_long_nl_queries_with_slashes_exploratory() {
        assert_eq!(
            derive_query_profile(
                "How are Rust symbol references extracted and linked to line/column positions in this project?"
            ),
            QueryProfile::Exploratory
        );
    }

    #[test]
    fn derive_query_profile_treats_bugfix_queries_separately() {
        assert_eq!(
            derive_query_profile("panic regression in semantic query pipeline"),
            QueryProfile::Bugfix
        );
    }

    #[test]
    fn graph_boost_scale_matches_profile_contract() {
        assert_eq!(graph_boost_scale(QueryProfile::Precise), 0.60);
        assert_eq!(graph_boost_scale(QueryProfile::Balanced), 0.75);
        assert_eq!(graph_boost_scale(QueryProfile::Exploratory), 0.55);
        assert_eq!(graph_boost_scale(QueryProfile::Bugfix), 0.70);
    }
}
