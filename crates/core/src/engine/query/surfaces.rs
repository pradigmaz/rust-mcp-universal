use std::time::Instant;

use anyhow::Result;

use super::super::{Engine, investigation};
use super::context_telemetry::{derive_chunk_telemetry, elapsed_ms};
use super::investigation_embed;
use crate::model::{
    ContextMode, ContextPackResult, ContextSelection, IndexTelemetry, QueryOptions,
    QuerySurfaceTimings,
};
use crate::report::{QueryReportBuildInput, build_query_report};

impl Engine {
    pub fn build_context_under_budget(
        &self,
        options: &QueryOptions,
        max_chars: usize,
        max_tokens: usize,
    ) -> Result<ContextSelection> {
        let execution = self.search_with_meta(options)?;
        self.context_for_hits_with_chunks(
            &options.query,
            &execution.hits,
            Some(&execution.chunk_by_path),
            options.context_mode,
            max_chars,
            max_tokens,
        )
    }

    pub fn build_context_pack(
        &self,
        options: &QueryOptions,
        mode: ContextMode,
        max_chars: usize,
        max_tokens: usize,
    ) -> Result<ContextPackResult> {
        let started = Instant::now();
        let mut timings = QuerySurfaceTimings::default();
        let mut options = options.clone();
        options.context_mode = Some(mode);
        let phase_started = Instant::now();
        let execution = self.search_with_meta(&options)?;
        timings.search_ms = elapsed_ms(phase_started);
        let phase_started = Instant::now();
        let context = self.context_for_hits_with_chunks(
            &options.query,
            &execution.hits,
            Some(&execution.chunk_by_path),
            options.context_mode,
            max_chars,
            max_tokens,
        )?;
        timings.context_ms = elapsed_ms(phase_started);
        let phase_started = Instant::now();
        let snapshot = investigation::shared_query_investigation_snapshot(
            self,
            &options.query,
            options.limit,
        )?;
        timings.investigation_ms = elapsed_ms(phase_started);
        timings.investigation = snapshot.timings;
        let phase_started = Instant::now();
        let investigation_hints = investigation_embed::format_investigation_hints(&snapshot);
        timings.format_ms = elapsed_ms(phase_started);
        timings.total_ms = elapsed_ms(started);
        Ok(ContextPackResult {
            mode,
            context,
            investigation_hints: Some(investigation_hints),
            timings: Some(timings),
        })
    }

    pub fn build_report(
        &self,
        options: &QueryOptions,
        max_chars: usize,
        max_tokens: usize,
    ) -> Result<crate::model::QueryReport> {
        let started = Instant::now();
        let mut timings = QuerySurfaceTimings::default();
        let phase_started = Instant::now();
        let execution = self.search_with_meta(options)?;
        timings.search_ms = elapsed_ms(phase_started);
        let phase_started = Instant::now();
        let context = self.context_for_hits_with_chunks(
            &options.query,
            &execution.hits,
            Some(&execution.chunk_by_path),
            options.context_mode,
            max_chars,
            max_tokens,
        )?;
        timings.context_ms = elapsed_ms(phase_started);
        let (chunk_coverage, chunk_source) = derive_chunk_telemetry(&context);
        let status = self.index_status()?;
        let investigation_summary = if options.detailed {
            let phase_started = Instant::now();
            let snapshot = investigation::shared_query_investigation_snapshot(
                self,
                &options.query,
                options.limit,
            )?;
            timings.investigation_ms = elapsed_ms(phase_started);
            timings.investigation = snapshot.timings;
            Some(investigation_embed::format_investigation_summary(&snapshot))
        } else {
            None
        };
        let phase_started = Instant::now();
        let mut report = build_query_report(
            &self.project_root,
            QueryReportBuildInput {
                shortlist: &execution.hits,
                context: &context,
                max_tokens,
                privacy_mode: options.privacy_mode,
                resolved_mode: execution.resolved_mode,
                mode_source: execution.mode_source,
                semantic_requested: options.semantic,
                semantic_outcome: execution.semantic_outcome,
                explain_entries: &execution.explain_entries,
                stage_counts: Some(execution.stage_counts),
                index_telemetry: IndexTelemetry {
                    last_index_lock_wait_ms: status.last_index_lock_wait_ms,
                    last_embedding_cache_hits: status.last_embedding_cache_hits,
                    last_embedding_cache_misses: status.last_embedding_cache_misses,
                    chunk_coverage,
                    chunk_source,
                },
                investigation_summary,
            },
        )?;
        timings.format_ms = elapsed_ms(phase_started);
        timings.total_ms = elapsed_ms(started);
        if options.detailed {
            report.timings = Some(timings);
        }
        Ok(report)
    }
}
