use super::*;

#[test]
fn empty_batch_request_returns_invalid_request() {
    let mut state = default_state();

    let raw = "[]";
    let response = expect_single_response(raw, &mut state);
    assert_eq!(response.error.expect("error expected").code, -32600);
}

#[test]
fn batch_request_returns_batch_response() {
    let mut state = default_state();

    let raw = r#"[{"jsonrpc":"2.0","id":1,"method":"initialize"},{"jsonrpc":"2.0","id":2,"method":"ping"}]"#;
    let response = process_raw_message(raw, &mut state).expect("response expected");
    assert_eq!(response.error.expect("error expected").code, -32600);
}

#[test]
fn batch_notifications_without_ids_yield_no_response() {
    let mut state = default_state();

    let raw = r#"[{"jsonrpc":"2.0","method":"notifications/initialized"},{"jsonrpc":"2.0","method":"ping"}]"#;
    let response = process_raw_message(raw, &mut state).expect("response expected");
    assert_eq!(response.error.expect("error expected").code, -32600);
}
