use std::fs;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

fn binary_path() -> String {
    env!("CARGO_BIN_EXE_rmu-mcp-server").to_string()
}

fn initialize_payload(id: i64) -> String {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "e2e", "version": "0.0.1"}
        }
    })
    .to_string()
}

fn framed_message(payload: &str, extra_headers: &[&str]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for header in extra_headers {
        bytes.extend_from_slice(header.as_bytes());
        bytes.extend_from_slice(b"\r\n");
    }
    bytes.extend_from_slice(format!("Content-Length: {}\r\n\r\n", payload.len()).as_bytes());
    bytes.extend_from_slice(payload.as_bytes());
    bytes
}

fn read_framed_response(stdout: &mut impl Read) -> Value {
    let mut headers = Vec::new();
    let mut byte = [0_u8; 1];
    while !headers.ends_with(b"\r\n\r\n") {
        stdout
            .read_exact(&mut byte)
            .expect("must read frame header");
        headers.push(byte[0]);
    }
    let header_text = String::from_utf8(headers).expect("header should be utf-8");
    let content_length = header_text
        .lines()
        .find_map(|line| line.strip_prefix("Content-Length: "))
        .expect("Content-Length header must exist")
        .parse::<usize>()
        .expect("Content-Length must be numeric");
    let mut body = vec![0_u8; content_length];
    stdout.read_exact(&mut body).expect("must read frame body");
    serde_json::from_slice(&body).expect("response body must be json")
}

fn temp_project_dir(prefix: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be monotonic")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{nanos}"))
}

fn expect_tool_result_hits(response: &Value) -> &[Value] {
    response["result"]["structuredContent"]["hits"]
        .as_array()
        .expect("result.structuredContent.hits should be array")
}

#[test]
fn malformed_content_length_does_not_execute_trailing_body() {
    let mut child = Command::new(binary_path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("server must spawn");

    {
        let stdin = child.stdin.as_mut().expect("stdin piped");
        stdin
            .write_all(format!("Content-Length: x\r\n\r\n{}", initialize_payload(2)).as_bytes())
            .expect("must write malformed frame");
    }

    let response = read_framed_response(child.stdout.as_mut().expect("stdout piped"));
    assert_eq!(response["error"]["code"], json!(-32700));

    drop(child.stdin.take());
    let mut remaining = Vec::new();
    child
        .stdout
        .as_mut()
        .expect("stdout piped")
        .read_to_end(&mut remaining)
        .expect("must read remaining output");
    assert!(
        remaining.is_empty(),
        "server must not emit a second response"
    );

    let status = child.wait().expect("process should exit");
    assert!(status.success());
}

#[test]
fn unknown_headers_do_not_break_valid_framed_request() {
    let mut child = Command::new(binary_path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("server must spawn");

    let payload = initialize_payload(1);
    child
        .stdin
        .as_mut()
        .expect("stdin piped")
        .write_all(&framed_message(&payload, &["Foo: bar", "X-Trace-Id: abc"]))
        .expect("must write request");

    let response = read_framed_response(child.stdout.as_mut().expect("stdout piped"));
    assert_eq!(response["id"], json!(1));
    assert_eq!(response["result"]["protocolVersion"], json!("2025-06-18"));

    drop(child.stdin.take());
    let status = child.wait().expect("process should exit");
    assert!(status.success());
}

#[test]
fn shutdown_then_exit_terminates_cleanly() {
    let mut child = Command::new(binary_path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("server must spawn");

    let stdin = child.stdin.as_mut().expect("stdin piped");
    stdin
        .write_all(&framed_message(&initialize_payload(1), &[]))
        .expect("must write initialize");
    let init_response = read_framed_response(child.stdout.as_mut().expect("stdout piped"));
    assert_eq!(init_response["id"], json!(1));

    stdin
        .write_all(&framed_message(
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            &[],
        ))
        .expect("must write initialized notification");
    stdin
        .write_all(&framed_message(
            r#"{"jsonrpc":"2.0","id":2,"method":"shutdown"}"#,
            &[],
        ))
        .expect("must write shutdown");

    let shutdown_response = read_framed_response(child.stdout.as_mut().expect("stdout piped"));
    assert_eq!(shutdown_response["id"], json!(2));
    assert_eq!(shutdown_response["result"], json!({}));

    stdin
        .write_all(&framed_message(r#"{"jsonrpc":"2.0","method":"exit"}"#, &[]))
        .expect("must write exit notification");
    drop(child.stdin.take());

    let status = child.wait().expect("process should exit");
    assert!(status.success());
}

#[test]
fn navigation_consumer_path_uses_symbol_lookup_v2_hits_envelope() {
    let project_dir = temp_project_dir("rmu-mcp-stdio-navigation-v2");
    fs::create_dir_all(project_dir.join("src")).expect("create src");
    fs::write(
        project_dir.join("src/main.rs"),
        "fn stdio_lookup_symbol_target() {}\nfn main() { stdio_lookup_symbol_target(); }\n",
    )
    .expect("write fixture");

    let mut child = Command::new(binary_path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("server must spawn");

    let stdin = child.stdin.as_mut().expect("stdin piped");
    stdin
        .write_all(&framed_message(&initialize_payload(1), &[]))
        .expect("must write initialize");
    let init_response = read_framed_response(child.stdout.as_mut().expect("stdout piped"));
    assert_eq!(init_response["id"], json!(1));

    stdin
        .write_all(&framed_message(
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            &[],
        ))
        .expect("must write initialized notification");

    let set_project_path = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "set_project_path",
            "arguments": {
                "project_path": project_dir.display().to_string()
            }
        }
    })
    .to_string();
    stdin
        .write_all(&framed_message(&set_project_path, &[]))
        .expect("must write set_project_path");
    let set_project_path_response =
        read_framed_response(child.stdout.as_mut().expect("stdout piped"));
    assert_eq!(set_project_path_response["id"], json!(2));
    assert_eq!(set_project_path_response["result"]["isError"], json!(false));

    let symbol_lookup = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "symbol_lookup_v2",
            "arguments": {
                "name": "stdio_lookup_symbol_target",
                "limit": 5,
                "auto_index": true
            }
        }
    })
    .to_string();
    stdin
        .write_all(&framed_message(&symbol_lookup, &[]))
        .expect("must write symbol_lookup_v2");
    let lookup_response = read_framed_response(child.stdout.as_mut().expect("stdout piped"));
    assert_eq!(lookup_response["id"], json!(3));
    assert_eq!(lookup_response["result"]["isError"], json!(false));
    let hits = expect_tool_result_hits(&lookup_response);
    assert!(hits.iter().any(|hit| {
        hit["name"] == json!("stdio_lookup_symbol_target") && hit["path"] == json!("src/main.rs")
    }));

    stdin
        .write_all(&framed_message(
            r#"{"jsonrpc":"2.0","id":4,"method":"shutdown"}"#,
            &[],
        ))
        .expect("must write shutdown");
    let shutdown_response = read_framed_response(child.stdout.as_mut().expect("stdout piped"));
    assert_eq!(shutdown_response["id"], json!(4));
    assert_eq!(shutdown_response["result"], json!({}));

    stdin
        .write_all(&framed_message(r#"{"jsonrpc":"2.0","method":"exit"}"#, &[]))
        .expect("must write exit notification");
    drop(child.stdin.take());

    let status = child.wait().expect("process should exit");
    assert!(status.success());

    let _ = fs::remove_dir_all(project_dir);
}
