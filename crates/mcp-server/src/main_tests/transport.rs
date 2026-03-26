use std::io::{self, BufReader};
use std::path::PathBuf;

use crate::{
    ServerState, WireMode, process_raw_message, read_framed_message, run_stdio_server,
    write_framed_message,
};

#[test]
fn framed_transport_roundtrip() {
    let payload = r#"{"jsonrpc":"2.0","id":1}"#;
    let mut framed = Vec::new();
    write_framed_message(&mut framed, payload).expect("must write frame");

    let mut reader = BufReader::new(framed.as_slice());
    let (decoded, mode) = read_framed_message(&mut reader)
        .expect("must read frame")
        .expect("must contain message");

    assert_eq!(decoded, payload);
    assert_eq!(mode, WireMode::Framed);
}

#[test]
fn read_framed_message_requires_content_length() {
    let bytes = b"Content-Type: application/json\r\n\r\n{}";
    let mut reader = BufReader::new(&bytes[..]);
    let err = read_framed_message(&mut reader).expect_err("must fail without content length");
    assert_eq!(err.kind(), io::ErrorKind::InvalidData);
}

#[test]
fn read_framed_message_accepts_single_line_json_without_headers() {
    let bytes = br#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#;
    let mut reader = BufReader::new(&bytes[..]);
    let (decoded, mode) = read_framed_message(&mut reader)
        .expect("must read json line")
        .expect("must contain message");

    assert_eq!(decoded, r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#);
    assert_eq!(mode, WireMode::LineJson);
}

#[test]
fn read_framed_message_accepts_non_object_json_lines_without_headers() {
    for raw in ["[]", "42"] {
        let mut reader = BufReader::new(raw.as_bytes());
        let (decoded, mode) = read_framed_message(&mut reader)
            .expect("must read json line")
            .expect("must contain message");
        assert_eq!(decoded, raw);
        assert_eq!(mode, WireMode::LineJson);
    }
}

#[test]
fn read_framed_message_accepts_single_line_json_without_waiting_for_eof() {
    let first = r#"{"jsonrpc":"2.0","id":0,"method":"initialize"}"#;
    let second = r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#;
    let stream = format!("{first}\nContent-Length: {}\r\n\r\n{second}", second.len());
    let mut reader = BufReader::new(stream.as_bytes());

    let (decoded_first, mode_first) = read_framed_message(&mut reader)
        .expect("must read first message")
        .expect("must contain message");
    assert_eq!(decoded_first, first);
    assert_eq!(mode_first, WireMode::LineJson);

    let (decoded_second, mode_second) = read_framed_message(&mut reader)
        .expect("must read second message")
        .expect("must contain message");
    assert_eq!(decoded_second, second);
    assert_eq!(mode_second, WireMode::Framed);
}

#[test]
fn line_json_batch_request_still_returns_invalid_request_code() {
    let mut reader = BufReader::new("[]".as_bytes());
    let (raw, mode) = read_framed_message(&mut reader)
        .expect("must read json line")
        .expect("must contain message");
    assert_eq!(mode, WireMode::LineJson);

    let mut state = ServerState::new(Some(PathBuf::from(".")), None);
    let response = process_raw_message(&raw, &mut state).expect("response expected");
    assert_eq!(response.error.expect("error expected").code, -32600);
}

#[test]
fn read_framed_message_accepts_case_insensitive_content_length_header() {
    let payload = r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#;
    let framed = format!(
        "content-length: {}\r\ncontent-type: application/json\r\n\r\n{}",
        payload.len(),
        payload
    );
    let mut reader = BufReader::new(framed.as_bytes());
    let (decoded, mode) = read_framed_message(&mut reader)
        .expect("must read frame")
        .expect("must contain payload");
    assert_eq!(decoded, payload);
    assert_eq!(mode, WireMode::Framed);
}

#[test]
fn read_framed_message_keeps_invalid_first_line_for_line_json_parse_error() {
    let stream = "not-json\n{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n";
    let mut reader = BufReader::new(stream.as_bytes());

    let (decoded, mode) = read_framed_message(&mut reader)
        .expect("must read first line")
        .expect("must contain payload");
    assert_eq!(decoded, "not-json");
    assert_eq!(mode, WireMode::LineJson);
}

#[test]
fn read_framed_message_treats_header_like_first_line_as_framed_mode() {
    let stream = "trace-id: abc\n{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n";
    let mut reader = BufReader::new(stream.as_bytes());
    let err = read_framed_message(&mut reader).expect_err("header-like lines enter framed mode");
    assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    assert!(err.to_string().contains("invalid framed header line"));
}

#[test]
fn read_framed_message_accepts_unknown_headers_when_content_length_is_valid() {
    let payload = r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#;
    let framed = format!(
        "Foo: bar\r\nContent-Length: {}\r\n\r\n{}",
        payload.len(),
        payload
    );
    let mut reader = BufReader::new(framed.as_bytes());
    let (decoded, mode) = read_framed_message(&mut reader)
        .expect("must read frame")
        .expect("must contain payload");
    assert_eq!(decoded, payload);
    assert_eq!(mode, WireMode::Framed);
}

#[test]
fn read_framed_message_treats_supported_header_without_colon_as_line_json_payload() {
    let stream = "Content-Length 10\n{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n";
    let mut reader = BufReader::new(stream.as_bytes());
    let (decoded, mode) = read_framed_message(&mut reader)
        .expect("must keep malformed first line as payload")
        .expect("must contain payload");
    assert_eq!(decoded, "Content-Length 10");
    assert_eq!(mode, WireMode::LineJson);
}

#[test]
fn read_framed_message_drops_invalid_frame_body_before_next_message() {
    let dropped = r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#;
    let next = r#"{"jsonrpc":"2.0","id":2,"method":"ping"}"#;
    let stream = format!(
        "Content-Length: {}\r\nContent-Length: {}\r\n\r\n{dropped}\n{next}\n",
        dropped.len(),
        dropped.len()
    );
    let mut reader = BufReader::new(stream.as_bytes());

    let err = read_framed_message(&mut reader).expect_err("must reject framed message");
    assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    assert!(err.to_string().contains("duplicate Content-Length"));

    let (decoded, mode) = read_framed_message(&mut reader)
        .expect("must read next message")
        .expect("must contain payload");
    assert_eq!(decoded, next);
    assert_eq!(mode, WireMode::LineJson);
}

#[test]
fn read_framed_message_accepts_utf8_bom_json_line() {
    let stream = "\u{feff}{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\"}\n";
    let mut reader = BufReader::new(stream.as_bytes());
    let (decoded, mode) = read_framed_message(&mut reader)
        .expect("must read json line")
        .expect("must contain payload");
    assert_eq!(decoded, r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#);
    assert_eq!(mode, WireMode::LineJson);
}

#[test]
fn read_framed_message_rejects_excessive_content_length() {
    let oversized = 8 * 1024 * 1024 + 1;
    let framed = format!("Content-Length: {oversized}\r\n\r\n");
    let mut reader = BufReader::new(framed.as_bytes());
    let err = read_framed_message(&mut reader).expect_err("must reject oversized frame");
    assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    assert!(err.to_string().contains("exceeds maximum supported size"));
}

#[test]
fn framed_transport_uses_utf8_byte_length() {
    let payload = r#"{"jsonrpc":"2.0","id":1,"result":{"text":"РџСЂРёРІРµС‚"}}"#;
    let mut framed = Vec::new();
    write_framed_message(&mut framed, payload).expect("must write frame");
    let framed_text = String::from_utf8(framed).expect("frame should be utf-8");
    let header = framed_text
        .lines()
        .next()
        .expect("frame should contain header");
    let reported = header
        .strip_prefix("Content-Length: ")
        .expect("content length header")
        .parse::<usize>()
        .expect("header should be number");
    assert_eq!(reported, payload.len());
}

#[test]
fn run_stdio_server_returns_framed_parse_error_for_header_like_first_line() {
    let mut reader = BufReader::new("trace-id: abc\n".as_bytes());
    let mut writer = Vec::new();
    let mut state = ServerState::new(Some(PathBuf::from(".")), None);

    run_stdio_server(&mut reader, &mut writer, &mut state).expect("stdio server should return");
    let output = String::from_utf8(writer).expect("writer should be utf-8");
    assert!(output.starts_with("Content-Length: "));
    let (_, payload) = output
        .split_once("\r\n\r\n")
        .expect("framed output should contain header separator");
    let response: serde_json::Value =
        serde_json::from_str(payload).expect("framed payload should be valid json");
    assert_eq!(response["error"]["code"], serde_json::json!(-32700));
}

#[test]
fn run_stdio_server_keeps_recoverable_framed_parse_errors_framed() {
    let dropped = r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#;
    let stream = format!(
        "Content-Length: {}\r\nContent-Length: {}\r\n\r\n{dropped}",
        dropped.len(),
        dropped.len()
    );
    let mut reader = BufReader::new(stream.as_bytes());
    let mut writer = Vec::new();
    let mut state = ServerState::new(Some(PathBuf::from(".")), None);

    run_stdio_server(&mut reader, &mut writer, &mut state).expect("stdio server should return");
    let output = String::from_utf8(writer).expect("writer should be utf-8");
    assert!(output.starts_with("Content-Length: "));
    let (_, payload) = output
        .split_once("\r\n\r\n")
        .expect("framed output should contain header separator");
    let response: serde_json::Value =
        serde_json::from_str(payload).expect("framed payload should be valid json");
    assert_eq!(response["error"]["code"], serde_json::json!(-32700));
}
