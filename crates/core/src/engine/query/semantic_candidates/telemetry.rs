#[derive(Debug, Clone, Copy)]
pub(super) struct RetrievalTelemetry {
    pub(super) semantic_fallback: bool,
}

impl RetrievalTelemetry {
    pub(super) const fn ann() -> Self {
        Self {
            semantic_fallback: false,
        }
    }

    pub(super) const fn fallback() -> Self {
        Self {
            semantic_fallback: true,
        }
    }
}
