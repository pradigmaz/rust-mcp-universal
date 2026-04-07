use crate::quality::LayeringFacts;

#[derive(Debug, Clone)]
pub(crate) struct LayeringFactsSummary {
    pub(crate) edge_count: i64,
    pub(crate) message: String,
}

impl LayeringFacts {
    pub(crate) fn violation_summary(&self) -> Option<LayeringFactsSummary> {
        let edge_count = self
            .forbidden_edge_count
            .saturating_add(self.out_of_direction_edge_count)
            .saturating_add(self.unmatched_edge_count);
        self.primary_message
            .as_ref()
            .filter(|message| !message.trim().is_empty() && edge_count > 0)
            .map(|message| LayeringFactsSummary {
                edge_count,
                message: if edge_count > 1 {
                    format!("{message} ({edge_count} edge(s))")
                } else {
                    message.clone()
                },
            })
    }
}
