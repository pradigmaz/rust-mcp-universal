use super::*;

#[test]
fn parse_error_uses_standard_code() {
    let response = parse_error_response("invalid".to_string());
    assert_eq!(response.error.expect("error expected").code, -32700);
}

#[test]
fn request_without_jsonrpc_is_rejected_as_invalid_request() {
    let mut state = default_state();
    let raw = r#"{"id":1,"method":"initialize","params":{}}"#;
    let response = expect_single_response(raw, &mut state);
    assert_eq!(response.error.expect("error expected").code, -32600);
    assert_eq!(response.id, Some(json!(1)));
}

#[test]
fn unknown_method_uses_method_not_found_and_preserves_id() {
    let mut state = default_state();
    let req = RpcRequest {
        jsonrpc: Some("2.0".to_string()),
        id: Some(json!(7)),
        method: "unknown/method".to_string(),
        params: None,
    };

    let response = handle_request(req, &mut state);
    assert_eq!(response.id, Some(json!(7)));
    assert_eq!(response.error.expect("error expected").code, -32601);
}

#[test]
fn invalid_request_keeps_id_when_available() {
    let mut state = default_state();

    let raw = r#"{"jsonrpc":"2.0","id":"abc","method":1}"#;
    let response = expect_single_response(raw, &mut state);
    assert_eq!(response.id, Some(json!("abc")));
    assert_eq!(response.error.expect("error expected").code, -32600);
}

#[test]
fn invalid_request_rejects_non_scalar_id_types() {
    let mut state = default_state();

    for raw in [
        r#"{"jsonrpc":"2.0","id":true,"method":"initialize"}"#,
        r#"{"jsonrpc":"2.0","id":[1],"method":"initialize"}"#,
        r#"{"jsonrpc":"2.0","id":{"nested":1},"method":"initialize"}"#,
    ] {
        let response = expect_single_response(raw, &mut state);
        assert_eq!(response.id, None);
        assert_eq!(response.error.expect("error expected").code, -32600);
    }
}

#[test]
fn malformed_json_uses_parse_error_with_null_id() {
    let mut state = default_state();

    let raw = r#"{"jsonrpc":"2.0","id":1,"method":"initialize""#;
    let response = expect_single_response(raw, &mut state);
    assert_eq!(response.id, None);
    assert_eq!(response.error.expect("error expected").code, -32700);
}
