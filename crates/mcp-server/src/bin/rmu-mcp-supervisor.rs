use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result};
use serde_json::Value;

struct Frame {
    raw: Vec<u8>,
    body: Vec<u8>,
}

struct ChildHandle {
    child: Child,
    stdin: ChildStdin,
}

fn main() -> Result<()> {
    let repo_root = repo_root()?;
    let trigger_path = repo_root.join("target/runtime/supervisor/reload.trigger");
    let mut trigger_seen = modified_time(&trigger_path);
    let suppress = Arc::new(AtomicUsize::new(0));
    let (parent_tx, parent_rx) = mpsc::channel::<Frame>();
    let (child_tx, child_rx) = mpsc::channel::<Vec<u8>>();

    thread::spawn(move || read_parent_stdin(parent_tx));
    thread::spawn(move || write_parent_stdout(child_rx));

    let mut child = spawn_child(&repo_root, child_tx.clone(), Arc::clone(&suppress))?;
    let mut initialize: Option<Vec<u8>> = None;
    let mut initialized: Option<Vec<u8>> = None;
    let mut set_project_path: Option<Vec<u8>> = None;

    loop {
        if reload_requested(&trigger_path, &mut trigger_seen) || child_exited(&mut child.child)? {
            child = reload_child(
                &repo_root,
                child,
                child_tx.clone(),
                Arc::clone(&suppress),
                [&initialize, &initialized, &set_project_path],
            )?;
        }

        let frame = match parent_rx.recv_timeout(Duration::from_millis(200)) {
            Ok(frame) => frame,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        };

        let method = method_name(&frame.body);
        if method.as_deref() == Some("initialize") {
            initialize = Some(frame.raw.clone());
        } else if method.as_deref() == Some("notifications/initialized") {
            initialized = Some(frame.raw.clone());
        } else if is_set_project_path(&frame.body) {
            set_project_path = Some(frame.raw.clone());
        }

        child.stdin.write_all(&frame.raw)?;
        child.stdin.flush()?;

        if method.as_deref() == Some("exit") {
            let _ = child.child.wait();
            break;
        }
    }

    let _ = child.child.kill();
    let _ = child.child.wait();
    Ok(())
}

fn repo_root() -> Result<PathBuf> {
    if let Ok(value) = std::env::var("RMU_REPO_ROOT") {
        return Ok(PathBuf::from(value));
    }
    Ok(std::env::current_dir()?)
}

fn spawn_child(
    repo_root: &Path,
    output_tx: mpsc::Sender<Vec<u8>>,
    suppress: Arc<AtomicUsize>,
) -> Result<ChildHandle> {
    let child_path = publish_child_binary(repo_root)?;
    let mut child = Command::new(&child_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("failed to spawn {}", child_path.display()))?;
    let stdout = child.stdout.take().context("child stdout must be piped")?;
    let stdin = child.stdin.take().context("child stdin must be piped")?;
    thread::spawn(move || forward_child_stdout(stdout, output_tx, suppress));
    Ok(ChildHandle { child, stdin })
}

fn reload_child(
    repo_root: &Path,
    mut old: ChildHandle,
    output_tx: mpsc::Sender<Vec<u8>>,
    suppress: Arc<AtomicUsize>,
    replay: [&Option<Vec<u8>>; 3],
) -> Result<ChildHandle> {
    let _ = old.child.kill();
    let _ = old.child.wait();

    let mut child = spawn_child(repo_root, output_tx, Arc::clone(&suppress))?;
    let replay_frames: Vec<&Vec<u8>> = replay.iter().filter_map(|item| item.as_ref()).collect();
    let response_count = replay_frames
        .iter()
        .filter(|frame| frame_has_id(frame))
        .count();
    suppress.fetch_add(response_count, Ordering::SeqCst);
    for frame in replay_frames {
        child.stdin.write_all(frame)?;
        child.stdin.flush()?;
    }
    Ok(child)
}

fn publish_child_binary(repo_root: &Path) -> Result<PathBuf> {
    let source = [
        repo_root.join("target/release/rmu-mcp-server.exe"),
        repo_root.join("target/debug/rmu-mcp-server.exe"),
    ]
    .into_iter()
    .find(|path| path.exists())
    .context("build rmu-mcp-server before starting supervisor")?;
    let runtime_dir = repo_root.join("target/runtime/supervisor");
    fs::create_dir_all(&runtime_dir)?;
    cleanup_old_children(&runtime_dir);
    let stamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis());
    let target = runtime_dir.join(format!(
        "rmu-mcp-server-child-{}-{stamp}.exe",
        std::process::id()
    ));
    fs::copy(&source, &target)
        .with_context(|| format!("failed to publish {}", target.display()))?;
    Ok(target)
}

fn cleanup_old_children(runtime_dir: &Path) {
    let Ok(entries) = fs::read_dir(runtime_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if name.starts_with("rmu-mcp-server-child-") && name.ends_with(".exe") {
            let _ = fs::remove_file(path);
        }
    }
}

fn read_parent_stdin(tx: mpsc::Sender<Frame>) {
    let mut stdin = io::stdin().lock();
    while let Ok(Some(frame)) = read_frame(&mut stdin) {
        if tx.send(frame).is_err() {
            break;
        }
    }
}

fn write_parent_stdout(rx: mpsc::Receiver<Vec<u8>>) {
    let mut stdout = io::stdout().lock();
    for frame in rx {
        if stdout
            .write_all(&frame)
            .and_then(|()| stdout.flush())
            .is_err()
        {
            break;
        }
    }
}

fn forward_child_stdout(
    mut stdout: impl Read,
    tx: mpsc::Sender<Vec<u8>>,
    suppress: Arc<AtomicUsize>,
) {
    while let Ok(Some(frame)) = read_frame(&mut stdout) {
        if suppress.load(Ordering::SeqCst) > 0 {
            suppress.fetch_sub(1, Ordering::SeqCst);
            continue;
        }
        if tx.send(frame.raw).is_err() {
            break;
        }
    }
}

fn read_frame(reader: &mut impl Read) -> io::Result<Option<Frame>> {
    let mut header = Vec::new();
    let mut byte = [0_u8; 1];
    loop {
        match reader.read_exact(&mut byte) {
            Ok(()) => header.push(byte[0]),
            Err(err) if err.kind() == io::ErrorKind::UnexpectedEof && header.is_empty() => {
                return Ok(None);
            }
            Err(err) => return Err(err),
        }
        if header.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    let header_text = String::from_utf8_lossy(&header);
    let length = header_text
        .lines()
        .find_map(|line| line.strip_prefix("Content-Length: "))
        .and_then(|value| value.parse::<usize>().ok())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing Content-Length"))?;
    let mut body = vec![0_u8; length];
    reader.read_exact(&mut body)?;
    let mut raw = header;
    raw.extend_from_slice(&body);
    Ok(Some(Frame { raw, body }))
}

fn method_name(body: &[u8]) -> Option<String> {
    let value: Value = serde_json::from_slice(body).ok()?;
    value.get("method")?.as_str().map(ToOwned::to_owned)
}

fn is_set_project_path(body: &[u8]) -> bool {
    let Ok(value) = serde_json::from_slice::<Value>(body) else {
        return false;
    };
    value.get("method").and_then(Value::as_str) == Some("tools/call")
        && value
            .get("params")
            .and_then(|params| params.get("name"))
            .and_then(Value::as_str)
            == Some("set_project_path")
}

fn frame_has_id(frame: &[u8]) -> bool {
    let Some(body_start) = frame.windows(4).position(|window| window == b"\r\n\r\n") else {
        return false;
    };
    serde_json::from_slice::<Value>(&frame[body_start + 4..])
        .ok()
        .and_then(|value| value.get("id").cloned())
        .is_some()
}

fn reload_requested(trigger_path: &Path, seen: &mut Option<SystemTime>) -> bool {
    let current = modified_time(trigger_path);
    let changed = current.is_some() && current > *seen;
    if changed {
        *seen = current;
    }
    changed
}

fn modified_time(path: &Path) -> Option<SystemTime> {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
}

fn child_exited(child: &mut Child) -> Result<bool> {
    match child.try_wait()? {
        Some(_) => Ok(true),
        None => Ok(false),
    }
}
