#![allow(dead_code)]

use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

pub fn temp_root(prefix: &str) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("obsidian-memory-mcp-tests-{prefix}-{suffix}"));
    std::fs::create_dir_all(&root).expect("create temp root");
    root
}

pub fn temp_root_on_workspace_mount(prefix: &str) -> PathBuf {
    #[cfg(windows)]
    {
        temp_root(prefix)
    }

    #[cfg(not(windows))]
    {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::current_dir()
            .expect("cwd")
            .join("target")
            .join("mcp-mounted")
            .join(format!("obsidian-memory-mcp-{prefix}-{suffix}"));
        std::fs::create_dir_all(&root).expect("create mounted temp root");
        root
    }
}

pub fn write_note(root: &Path, relative: &str, title: &str, node_type: &str, body: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create parent");
    }
    let normalized_type = node_type
        .trim()
        .replace(['-', ' '], "_")
        .to_ascii_lowercase();
    let slug = if relative == "_index.md" {
        "_index".to_string()
    } else {
        Path::new(relative)
            .file_stem()
            .and_then(|value| value.to_str())
            .expect("slug")
            .to_string()
    };
    let content = format!(
        "---\nid: {normalized_type}-{slug}\ntype: {node_type}\ntitle: {title}\nstatus: active\nproject: workspace\ncreated_at: 1\nupdated_at: 1\n---\n\n# {title}\n\n## Summary\n{body}\n\n## Observations\n\n## Relations\n\n## References\n"
    );
    std::fs::write(path, content).expect("write note");
}

pub fn ensure_canonical_layout(root: &Path) {
    for directory in [
        "architecture",
        "artifacts",
        "constraints",
        "decisions",
        "glossary",
        "modules",
        "progress",
        "risks",
        "tasks",
    ] {
        std::fs::create_dir_all(root.join(directory)).expect("create canonical dir");
    }
}

pub fn read_note(root: &Path, relative: &str) -> String {
    std::fs::read_to_string(root.join(relative)).expect("read note")
}

pub fn tool_success(response: &Value) -> &Value {
    assert_eq!(response["result"]["isError"], json!(false), "{response}");
    &response["result"]["structuredContent"]
}

pub fn tool_error(response: &Value) -> &Value {
    assert_eq!(response["result"]["isError"], json!(true), "{response}");
    &response["result"]["structuredContent"]
}

#[cfg(not(windows))]
pub fn windows_style_path(path: &Path) -> String {
    let canonical = std::fs::canonicalize(path).expect("canonical path");
    let raw = canonical.to_string_lossy().replace('\\', "/");
    let rest = raw
        .strip_prefix("/mnt/")
        .expect("mounted path should live under /mnt/<drive>");
    let (drive, tail) = rest.split_once('/').expect("drive tail");
    format!(
        "{}:\\{}",
        drive.to_ascii_uppercase(),
        tail.replace('/', "\\")
    )
}

#[cfg(not(windows))]
pub fn windows_file_uri(path: &Path) -> String {
    format!("file:///{}", windows_style_path(path).replace('\\', "/"))
}

pub struct ServerHarness {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    codex_home: PathBuf,
}

impl ServerHarness {
    pub fn spawn() -> Self {
        let codex_home = temp_root("codex-home");
        let mut child = Command::new(env!("CARGO_BIN_EXE_obsidian-memory-mcp-server"))
            .env("CODEX_HOME", &codex_home)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("spawn server");
        let stdin = child.stdin.take().expect("stdin");
        let stdout = BufReader::new(child.stdout.take().expect("stdout"));
        Self {
            child,
            stdin,
            stdout,
            codex_home,
        }
    }

    pub fn send_line(&mut self, payload: &Value) {
        let encoded = serde_json::to_string(payload).expect("encode");
        writeln!(self.stdin, "{encoded}").expect("write line");
        self.stdin.flush().expect("flush");
    }

    pub fn read_line(&mut self) -> Value {
        let mut line = String::new();
        self.stdout.read_line(&mut line).expect("read line");
        serde_json::from_str(line.trim_end()).expect("decode line json")
    }

    pub fn send_framed(&mut self, payload: &Value) {
        let encoded = serde_json::to_string(payload).expect("encode");
        write!(
            self.stdin,
            "Content-Length: {}\r\n\r\n{}",
            encoded.len(),
            encoded
        )
        .expect("write framed");
        self.stdin.flush().expect("flush");
    }

    pub fn read_framed(&mut self) -> Value {
        let mut content_length = None::<usize>;
        loop {
            let mut line = String::new();
            self.stdout.read_line(&mut line).expect("read header");
            let trimmed = line.trim_end_matches(['\r', '\n']);
            if trimmed.is_empty() {
                break;
            }
            if let Some(value) = trimmed.strip_prefix("Content-Length: ") {
                content_length = Some(value.parse::<usize>().expect("content length"));
            }
        }
        let length = content_length.expect("content length header");
        let mut body = vec![0_u8; length];
        self.stdout.read_exact(&mut body).expect("read body");
        serde_json::from_slice(&body).expect("decode framed json")
    }

    pub fn initialize_line(&mut self) {
        self.initialize_line_with_params(json!({}));
    }

    pub fn initialize_line_with_params(&mut self, params: Value) {
        let mut full_params = json!({
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {
                "name": "mcp-line-test",
                "version": "0.1.0"
            }
        });
        if let Some(extra) = params.as_object() {
            let object = full_params
                .as_object_mut()
                .expect("initialize params object");
            for (key, value) in extra {
                object.insert(key.clone(), value.clone());
            }
        }
        self.send_line(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": full_params
        }));
        let response = self.read_line();
        assert_eq!(response["result"]["protocolVersion"], json!("2025-06-18"));
        self.send_line(&json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }));
    }

    pub fn initialize_framed(&mut self) {
        self.send_framed(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {
                    "name": "mcp-framed-test",
                    "version": "0.1.0"
                }
            }
        }));
        let response = self.read_framed();
        assert_eq!(response["result"]["protocolVersion"], json!("2025-06-18"));
        self.send_framed(&json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }));
    }

    pub fn list_tools_line(&mut self, id: u64) -> Value {
        self.send_line(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/list"
        }));
        self.read_line()
    }

    pub fn call_line_tool(&mut self, id: u64, name: &str, arguments: Value) -> Value {
        self.send_line(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": {
                "name": name,
                "arguments": arguments
            }
        }));
        self.read_line()
    }

    pub fn shutdown_line(mut self) {
        self.send_line(&json!({
            "jsonrpc": "2.0",
            "id": 99,
            "method": "shutdown"
        }));
        let response = self.read_line();
        assert!(response["result"].is_object());
        self.send_line(&json!({
            "jsonrpc": "2.0",
            "method": "exit"
        }));
        let status = self.child.wait().expect("wait");
        assert!(status.success());
        let _ = std::fs::remove_dir_all(&self.codex_home);
    }

    pub fn shutdown_framed(mut self) {
        self.send_framed(&json!({
            "jsonrpc": "2.0",
            "id": 99,
            "method": "shutdown"
        }));
        let response = self.read_framed();
        assert!(response["result"].is_object());
        self.send_framed(&json!({
            "jsonrpc": "2.0",
            "method": "exit"
        }));
        let status = self.child.wait().expect("wait");
        assert!(status.success());
        let _ = std::fs::remove_dir_all(&self.codex_home);
    }

    pub fn codex_home(&self) -> &Path {
        &self.codex_home
    }
}
