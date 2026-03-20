mod boost;
mod helpers;
mod lexical;
mod scoring;
#[path = "search_db_unicode.rs"]
mod unicode;

pub(crate) use boost::{extract_tokens, graph_boost};
pub use lexical::{search_fts, search_like};

#[cfg(test)]
use helpers::escape_like_value;
#[cfg(test)]
use lexical::{like_prefilter_limit, like_scan_budget};
#[cfg(test)]
use scoring::{compare_hits_desc, keep_top_hits, like_score, path_match_boost};

#[cfg(test)]
#[path = "search_db_tests.rs"]
mod tests;
