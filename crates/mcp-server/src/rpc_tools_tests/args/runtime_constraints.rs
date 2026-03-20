use super::*;

type RuntimeConstraintCase = (&'static str, serde_json::Value, &'static str);

mod bootstrap_and_project_path;
mod general_numeric_bounds;
mod indexing_and_deletion;
mod query_benchmark;
mod runtime_flags_and_modes;
mod strict_argument_contracts;

#[test]
fn runtime_argument_constraints_reject_invalid_tool_args() {
    let mut cases: Vec<RuntimeConstraintCase> = Vec::new();
    cases.extend(general_numeric_bounds::cases());
    cases.extend(query_benchmark::cases());
    cases.extend(runtime_flags_and_modes::cases());
    cases.extend(indexing_and_deletion::cases());
    cases.extend(bootstrap_and_project_path::cases());
    cases.extend(strict_argument_contracts::cases());

    if usize::BITS >= 64 {
        let oversized = (i64::MAX as u128 + 1).to_string();
        let oversized_args: serde_json::Value =
            serde_json::from_str(&format!(r#"{{"query":"q","limit":{oversized}}}"#))
                .expect("oversized json payload should be valid");
        cases.push(("search_candidates", oversized_args, "`limit` <="));
    }

    for (tool_name, arguments, expected_message_fragment) in cases {
        assert_tool_args_error(tool_name, arguments, expected_message_fragment);
    }
}
