use anyhow::Result;

use crate::engine::Engine;
use crate::model::{
    SensitiveDataOptions, SensitiveDataResult, SignalMemoryEntry, SignalMemoryMarkRequest,
    SignalMemoryOptions, SignalMemoryResult,
};

#[path = "security/sensitive_data.rs"]
mod sensitive_data;

impl Engine {
    pub fn sensitive_data(&self, options: &SensitiveDataOptions) -> Result<SensitiveDataResult> {
        sensitive_data::scan_sensitive_data(self, options)
    }

    pub fn signal_memory(&self, options: &SignalMemoryOptions) -> Result<SignalMemoryResult> {
        crate::signal_memory::inspect_signal_memory(&self.project_root, options)
    }

    pub fn mark_signal_memory(
        &self,
        request: &SignalMemoryMarkRequest,
    ) -> Result<SignalMemoryEntry> {
        crate::signal_memory::mark_signal_memory(&self.project_root, request)
    }
}
