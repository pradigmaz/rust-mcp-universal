use super::{
    ProcessLiveness, REBUILD_LOCK_FORCE_RECLAIM_SECS, REBUILD_LOCK_ORPHAN_GRACE_SECS,
    REBUILD_LOCK_WAIT_TIMEOUT_MS, RebuildLockGuard, lock_path_for_db, parse_lock_metadata,
    parse_process_probe_output, process_liveness,
};
use std::fs;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

fn temp_dir(prefix: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be monotonic")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{nanos}"))
}

#[test]
fn drop_does_not_remove_lock_owned_by_another_token() {
    let root = temp_dir("rmu-rebuild-lock-token-check");
    fs::create_dir_all(&root).expect("create dir");
    let db_path = root.join("index.db");

    let guard = RebuildLockGuard::try_acquire(&db_path)
        .expect("lock acquisition should not fail")
        .expect("lock should be acquired");
    let lock_path = lock_path_for_db(&db_path);
    fs::write(
        &lock_path,
        b"pid=999\nlock_token=other-token\nstarted_at_utc=2000-01-01T00:00:00Z\n",
    )
    .expect("overwrite lock");

    drop(guard);
    assert!(lock_path.exists());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn try_acquire_creates_missing_lock_parent_directory() {
    let root = temp_dir("rmu-rebuild-lock-creates-parent");
    let db_path = root.join("nested/store/index.db");

    let guard = RebuildLockGuard::try_acquire(&db_path)
        .expect("lock acquisition should not fail")
        .expect("lock should be acquired");
    let lock_path = lock_path_for_db(&db_path);

    assert!(
        lock_path.parent().is_some_and(std::path::Path::exists),
        "lock parent should be created eagerly"
    );

    drop(guard);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn try_acquire_reclaims_orphan_malformed_lock_before_stale_window() {
    let root = temp_dir("rmu-rebuild-lock-malformed-before-stale-window");
    fs::create_dir_all(&root).expect("create dir");
    let db_path = root.join("index.db");
    let lock_path = lock_path_for_db(&db_path);
    fs::write(&lock_path, b"broken-lock-content").expect("write malformed lock");

    let stale_ts = SystemTime::now() - Duration::from_secs(REBUILD_LOCK_ORPHAN_GRACE_SECS + 5);
    let lock_file = std::fs::File::options()
        .write(true)
        .open(&lock_path)
        .expect("open lock file");
    lock_file.set_modified(stale_ts).expect("set modified");

    let guard = RebuildLockGuard::try_acquire(&db_path)
        .expect("lock acquisition should not fail")
        .expect("lock should be acquired after reclaim");
    drop(guard);
    assert!(!lock_path.exists());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn try_acquire_force_reclaims_very_old_lock_even_if_pid_alive() {
    let root = temp_dir("rmu-rebuild-lock-force-reclaim-old");
    fs::create_dir_all(&root).expect("create dir");
    let db_path = root.join("index.db");
    let lock_path = lock_path_for_db(&db_path);
    fs::write(
        &lock_path,
        format!(
            "pid={}\nlock_token=stale-token\nstarted_at_utc=2000-01-01T00:00:00Z\n",
            std::process::id()
        ),
    )
    .expect("write stale lock payload");

    let guard = RebuildLockGuard::try_acquire(&db_path)
        .expect("lock acquisition should not fail")
        .expect("lock should be acquired after force reclaim");
    drop(guard);
    assert!(!lock_path.exists());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn process_probe_parser_maps_known_tokens() {
    assert_eq!(parse_process_probe_output(b"alive"), ProcessLiveness::Alive);
    assert_eq!(parse_process_probe_output(b"dead\n"), ProcessLiveness::Dead);
    assert_eq!(
        parse_process_probe_output(b"unknown"),
        ProcessLiveness::Unknown
    );
    assert_eq!(parse_process_probe_output(b""), ProcessLiveness::Unknown);
}

#[test]
fn parse_lock_metadata_handles_partial_payload() {
    let metadata = parse_lock_metadata("pid=123\n");
    assert_eq!(metadata.pid, Some(123));
    assert!(metadata.started_at_utc.is_none());
}

#[test]
fn current_process_liveness_is_not_dead() {
    assert_ne!(process_liveness(std::process::id()), ProcessLiveness::Dead);
}

#[cfg(all(unix, not(target_os = "linux")))]
#[test]
fn non_linux_unix_pid_zero_is_unknown() {
    assert_eq!(
        super::process_probe::process_liveness_unix_fallback(0),
        ProcessLiveness::Unknown
    );
}

#[test]
fn try_acquire_reclaims_very_old_lock_without_started_at() {
    let root = temp_dir("rmu-rebuild-lock-force-reclaim-without-started-at");
    fs::create_dir_all(&root).expect("create dir");
    let db_path = root.join("index.db");
    let lock_path = lock_path_for_db(&db_path);
    fs::write(
        &lock_path,
        format!("pid={}\nlock_token=stale-token\n", std::process::id()),
    )
    .expect("write stale lock payload");

    let stale_ts = SystemTime::now() - Duration::from_secs(REBUILD_LOCK_FORCE_RECLAIM_SECS + 5);
    let lock_file = std::fs::File::options()
        .write(true)
        .open(&lock_path)
        .expect("open lock file");
    lock_file.set_modified(stale_ts).expect("set modified");

    let guard = RebuildLockGuard::try_acquire(&db_path)
        .expect("lock acquisition should not fail")
        .expect("lock should be acquired after force reclaim");
    drop(guard);
    assert!(!lock_path.exists());

    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn try_acquire_does_not_reclaim_unreadable_lock_file() {
    use std::os::unix::fs::PermissionsExt;

    let root = temp_dir("rmu-rebuild-lock-unreadable");
    fs::create_dir_all(&root).expect("create dir");
    let db_path = root.join("index.db");
    let lock_path = lock_path_for_db(&db_path);
    fs::write(&lock_path, b"broken-lock-content").expect("write lock");

    let stale_ts = SystemTime::now() - Duration::from_secs(REBUILD_LOCK_ORPHAN_GRACE_SECS + 5);
    let lock_file = std::fs::File::options()
        .write(true)
        .open(&lock_path)
        .expect("open lock file");
    lock_file.set_modified(stale_ts).expect("set modified");

    let mut permissions = fs::metadata(&lock_path).expect("metadata").permissions();
    permissions.set_mode(0o000);
    fs::set_permissions(&lock_path, permissions).expect("set unreadable permissions");

    let acquired = RebuildLockGuard::try_acquire(&db_path)
        .expect("try_acquire should not fail")
        .is_some();
    assert!(!acquired, "unreadable lock must not be reclaimed");

    let mut restore_permissions = fs::metadata(&lock_path).expect("metadata").permissions();
    restore_permissions.set_mode(0o600);
    let _ = fs::set_permissions(&lock_path, restore_permissions);
    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn lock_path_for_db_distinguishes_percent_and_raw_bytes() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let root = temp_dir("rmu-rebuild-lock-path-encoding");
    fs::create_dir_all(&root).expect("create dir");
    let utf8 = root.join("%FF.db");
    let raw = root.join(OsStr::from_bytes(b"\xFF.db"));

    assert_ne!(lock_path_for_db(&utf8), lock_path_for_db(&raw));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn acquire_times_out_under_lock_contention_with_bounded_wait() {
    let root = temp_dir("rmu-rebuild-lock-contention-timeout");
    fs::create_dir_all(&root).expect("create dir");
    let db_path = root.join("index.db");

    let held = RebuildLockGuard::acquire(&db_path).expect("initial lock acquire");
    let start = Instant::now();
    let err = RebuildLockGuard::acquire(&db_path)
        .expect_err("second acquire under contention must time out");
    let elapsed_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);

    assert!(
        err.to_string()
            .contains("timed out waiting for rebuild lock"),
        "unexpected error: {err}"
    );
    assert!(
        elapsed_ms >= REBUILD_LOCK_WAIT_TIMEOUT_MS,
        "waited too little under contention: elapsed_ms={elapsed_ms}, timeout_ms={REBUILD_LOCK_WAIT_TIMEOUT_MS}"
    );
    assert!(
        elapsed_ms <= REBUILD_LOCK_WAIT_TIMEOUT_MS + 8_000,
        "wait exceeded bounded timeout window: elapsed_ms={elapsed_ms}, timeout_ms={REBUILD_LOCK_WAIT_TIMEOUT_MS}"
    );

    drop(held);
    let _ = fs::remove_dir_all(root);
}
