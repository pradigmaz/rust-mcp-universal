use serde::{Deserialize, Serialize};

use super::investigation_cluster::ImplementationVariant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConceptSeedKind {
    Query,
    Symbol,
    Path,
    PathLine,
}

impl ConceptSeedKind {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "query" => Some(Self::Query),
            "symbol" => Some(Self::Symbol),
            "path" => Some(Self::Path),
            "path_line" | "path-line" => Some(Self::PathLine),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptSeed {
    pub seed: String,
    pub seed_kind: ConceptSeedKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SourceSpan {
    pub start_line: usize,
    pub end_line: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_column: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_column: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InvestigationAnchor {
    pub path: String,
    pub language: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolBodyItem {
    pub anchor: InvestigationAnchor,
    pub signature: String,
    pub body: String,
    pub span: SourceSpan,
    pub source_kind: String,
    pub resolution_kind: SymbolBodyResolutionKind,
    pub truncated: bool,
    pub confidence: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolBodyResolutionKind {
    ExactSymbolSpan,
    NearestIndexedLines,
    ChunkExcerptAnchor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolBodyAmbiguityStatus {
    None,
    MultipleExact,
    PartialOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteSegmentKind {
    Ui,
    ApiClient,
    Endpoint,
    Service,
    Crud,
    Query,
    Test,
    Migration,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteSegment {
    pub kind: RouteSegmentKind,
    pub path: String,
    pub language: String,
    pub evidence: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor_symbol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_span: Option<SourceSpan>,
    pub relation_kind: String,
    pub source_kind: String,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutePath {
    pub segments: Vec<RouteSegment>,
    pub total_hops: usize,
    pub total_weight: f32,
    pub collapsed_hops: usize,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteGap {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_kind: Option<RouteSegmentKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_kind: Option<RouteSegmentKind>,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_resolved_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintEvidence {
    pub constraint_kind: String,
    pub source_kind: String,
    pub path: String,
    pub line_start: usize,
    pub line_end: usize,
    pub excerpt: String,
    pub confidence: f32,
    pub normalized_key: String,
    // Legacy aliases kept for additive compatibility with existing contracts.
    pub kind: String,
    pub strength: String,
    pub scope: String,
    pub source_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_span: Option<SourceSpan>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub introduced_by: Option<String>,
    pub normalized_text: String,
}

impl ConstraintEvidence {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        constraint_kind: &str,
        source_kind: &str,
        path: String,
        line_start: usize,
        line_end: usize,
        excerpt: String,
        strength: &str,
        scope: String,
        introduced_by: Option<String>,
        confidence: f32,
        normalized_text: String,
    ) -> Self {
        let normalized_excerpt = excerpt.trim().to_ascii_lowercase();
        let normalized_key = format!(
            "{constraint_kind}:{source_kind}:{}",
            normalized_excerpt
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        );
        let source_span = Some(SourceSpan {
            start_line: line_start,
            end_line: line_end,
            start_column: Some(1),
            end_column: None,
        });
        Self {
            constraint_kind: constraint_kind.to_string(),
            source_kind: source_kind.to_string(),
            path: path.clone(),
            line_start,
            line_end,
            excerpt,
            confidence,
            normalized_key,
            kind: constraint_kind.to_string(),
            strength: strength.to_string(),
            scope,
            source_path: path,
            source_span,
            introduced_by,
            normalized_text,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolBodyResult {
    pub seed: ConceptSeed,
    pub items: Vec<SymbolBodyItem>,
    pub capability_status: String,
    pub unsupported_sources: Vec<String>,
    pub ambiguity_status: SymbolBodyAmbiguityStatus,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintEvidenceResult {
    pub seed: ConceptSeed,
    pub items: Vec<ConstraintEvidence>,
    pub capability_status: String,
    pub unsupported_sources: Vec<String>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteTraceResult {
    pub seed: ConceptSeed,
    pub best_route: RoutePath,
    pub alternate_routes: Vec<RoutePath>,
    pub unresolved_gaps: Vec<RouteGap>,
    pub capability_status: String,
    pub unsupported_sources: Vec<String>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxisObservation {
    pub variant_id: String,
    pub values: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DivergenceAxis {
    pub axis: String,
    pub values: Vec<AxisObservation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DivergenceSignal {
    pub severity: String,
    pub axis: String,
    pub evidence_strength: String,
    pub classification_reason: String,
    pub summary: String,
    pub variant_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DivergenceReport {
    pub surface_kind: String,
    pub seed: ConceptSeed,
    pub variants: Vec<ImplementationVariant>,
    pub consensus_axes: Vec<DivergenceAxis>,
    pub divergence_axes: Vec<DivergenceAxis>,
    pub divergence_signals: Vec<DivergenceSignal>,
    pub overall_severity: String,
    pub manual_review_required: bool,
    pub summary: String,
    pub shared_evidence: Vec<String>,
    pub unknowns: Vec<String>,
    pub missing_evidence: Vec<String>,
    pub recommended_followups: Vec<String>,
    pub overall_confidence: f32,
    pub capability_status: String,
    pub unsupported_sources: Vec<String>,
}
