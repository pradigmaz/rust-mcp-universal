use std::{fs, path::Path};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::model::{QueryBenchmarkCase, QueryBenchmarkDataset, QueryQrel};

use super::metrics::canonical_benchmark_path;

pub(super) fn load_benchmark_dataset(dataset_path: &Path) -> Result<QueryBenchmarkDataset> {
    let raw = fs::read_to_string(dataset_path).with_context(|| {
        format!(
            "failed to read benchmark dataset from {}",
            dataset_path.display()
        )
    })?;
    let parsed = serde_json::from_str::<RawDataset>(&raw).with_context(|| {
        format!(
            "failed to parse benchmark dataset json from {}",
            dataset_path.display()
        )
    })?;

    let mut queries = match parsed {
        RawDataset::Object { queries } => queries,
        RawDataset::Array(queries) => queries,
    };
    if queries.is_empty() {
        return Ok(QueryBenchmarkDataset {
            queries: Vec::new(),
        });
    }

    let mut normalized = Vec::with_capacity(queries.len());
    for raw_case in queries.drain(..) {
        let query = raw_case.query.trim().to_string();
        if query.is_empty() {
            bail!("benchmark dataset contains query with empty `query` field");
        }
        let qrels = normalize_qrels(raw_case);
        normalized.push(QueryBenchmarkCase { query, qrels });
    }

    Ok(QueryBenchmarkDataset {
        queries: normalized,
    })
}

fn normalize_qrels(raw_case: RawBenchmarkCase) -> Vec<QueryQrel> {
    if !raw_case.qrels.is_empty() {
        return raw_case
            .qrels
            .into_iter()
            .filter_map(|entry| match entry {
                RawQrel::Path(path) => canonical_benchmark_path(&path).map(|path| QueryQrel {
                    path,
                    relevance: 1.0,
                }),
                RawQrel::WithRelevance { path, relevance } => {
                    canonical_benchmark_path(&path).map(|path| QueryQrel {
                        path,
                        relevance: relevance.max(0.0),
                    })
                }
            })
            .collect();
    }

    raw_case
        .relevant_paths
        .into_iter()
        .filter_map(|path| canonical_benchmark_path(&path))
        .map(|path| QueryQrel {
            path,
            relevance: 1.0,
        })
        .collect()
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawDataset {
    Object { queries: Vec<RawBenchmarkCase> },
    Array(Vec<RawBenchmarkCase>),
}

#[derive(Debug, Deserialize)]
struct RawBenchmarkCase {
    query: String,
    #[serde(default, alias = "relevant", alias = "expected")]
    qrels: Vec<RawQrel>,
    #[serde(default, alias = "relevant_paths")]
    relevant_paths: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawQrel {
    Path(String),
    WithRelevance { path: String, relevance: f32 },
}
