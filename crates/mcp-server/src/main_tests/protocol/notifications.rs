use super::*;

#[test]
fn valid_notification_without_id_yields_no_response() {
    let mut state = default_state();

    let raw = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
    assert!(process_raw_message(raw, &mut state).is_none());
}
