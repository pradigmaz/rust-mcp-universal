use serde_json::Value;

use super::super::common::enum_schema;

pub(crate) fn agent_intent_mode_schema(description: &str) -> Value {
    enum_schema(
        description,
        &[
            "entrypoint_map",
            "test_map",
            "review_prep",
            "api_contract_map",
            "runtime_surface",
            "refactor_surface",
        ],
    )
}

pub(crate) fn bootstrap_profile_schema() -> Value {
    enum_schema(
        "Preferred bootstrap surface depth. Overrides legacy include flags when provided.",
        &["fast", "investigation_summary", "report", "full"],
    )
}
