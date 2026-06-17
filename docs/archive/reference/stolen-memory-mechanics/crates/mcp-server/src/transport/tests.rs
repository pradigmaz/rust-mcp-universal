use std::io::Cursor;

use super::{WireMode, read_framed_message, write_framed_message};

#[test]
fn reads_framed_messages() {
    let input = b"Content-Length: 25\r\n\r\n{\"jsonrpc\":\"2.0\",\"id\":1}\n";
    let mut reader = Cursor::new(input);

    let parsed = read_framed_message(&mut reader)
        .expect("framed")
        .expect("message");

    assert_eq!(parsed.1, WireMode::Framed);
    assert_eq!(parsed.0, "{\"jsonrpc\":\"2.0\",\"id\":1}\n");
}

#[test]
fn falls_back_to_line_json_when_first_line_is_not_header() {
    let input = b"{\"jsonrpc\":\"2.0\",\"id\":1}\n";
    let mut reader = Cursor::new(input);

    let parsed = read_framed_message(&mut reader)
        .expect("line json")
        .expect("message");

    assert_eq!(parsed.1, WireMode::LineJson);
    assert_eq!(parsed.0, "{\"jsonrpc\":\"2.0\",\"id\":1}");
}

#[test]
fn duplicate_content_length_is_recoverable_after_consuming_body() {
    let input = b"Content-Length: 2\r\nContent-Length: 2\r\n\r\n{}";
    let mut reader = Cursor::new(input);

    let err = read_framed_message(&mut reader).expect_err("duplicate header should fail");

    assert!(err.is_recoverable());
    assert!(err.message().contains("duplicate Content-Length header"));
}

#[test]
fn write_framed_message_prefixes_content_length() {
    let mut out = Vec::new();

    write_framed_message(&mut out, "{\"ok\":true}").expect("write framed");

    let rendered = String::from_utf8(out).expect("utf8");
    assert_eq!(rendered, "Content-Length: 11\r\n\r\n{\"ok\":true}");
}
