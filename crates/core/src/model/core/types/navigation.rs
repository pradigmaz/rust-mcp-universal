use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolMatch {
    pub path: String,
    pub name: String,
    pub kind: String,
    pub language: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub exact: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolReferenceHit {
    pub path: String,
    pub language: String,
    pub ref_count: usize,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub exact: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedFileHit {
    pub path: String,
    pub language: String,
    pub score: f32,
    pub dep_overlap: usize,
    pub ref_overlap: usize,
    pub symbol_overlap: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallPathEndpoint {
    pub input: String,
    pub resolved_path: String,
    pub kind: String,
    pub symbol: Option<String>,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallPathStep {
    pub from_path: String,
    pub to_path: String,
    pub edge_kind: String,
    pub raw_count: usize,
    pub weight: f32,
    pub evidence: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallPathExplain {
    pub algorithm: String,
    pub max_hops: usize,
    pub visited_nodes: usize,
    pub considered_edges: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallPathResult {
    pub from: CallPathEndpoint,
    pub to: CallPathEndpoint,
    pub found: bool,
    pub path: Vec<String>,
    pub steps: Vec<CallPathStep>,
    pub hops: usize,
    pub total_weight: f32,
    pub explain: CallPathExplain,
}
