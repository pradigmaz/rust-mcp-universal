#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
repo_root="$(cd -- "$script_dir/.." && pwd -P)"
binary_path="$repo_root/target/release/rmu-mcp-server"

get_latest_source_mtime() {
  local latest=0
  local path
  local candidate

  for path in \
    "$repo_root/Cargo.toml" \
    "$repo_root/Cargo.lock" \
    "$repo_root/crates/core" \
    "$repo_root/crates/mcp-server"
  do
    [[ -e "$path" ]] || continue

    if [[ -d "$path" ]]; then
      while IFS= read -r -d '' candidate; do
        local mtime
        mtime="$(stat -c %Y "$candidate")"
        if (( mtime > latest )); then
          latest="$mtime"
        fi
      done < <(find "$path" -type f -print0)
    else
      candidate="$(stat -c %Y "$path")"
      if (( candidate > latest )); then
        latest="$candidate"
      fi
    fi
  done

  printf '%s\n' "$latest"
}

rebuild_required() {
  if [[ ! -x "$binary_path" ]]; then
    return 0
  fi

  local binary_mtime
  local latest_source_mtime
  binary_mtime="$(stat -c %Y "$binary_path")"
  latest_source_mtime="$(get_latest_source_mtime)"
  (( latest_source_mtime > binary_mtime ))
}

stop_stale_server_processes() {
  local pid
  local exe_path
  local matched=()

  for proc_dir in /proc/[0-9]*; do
    pid="${proc_dir##*/}"
    [[ "$pid" != "$$" ]] || continue

    if [[ ! -L "$proc_dir/exe" ]]; then
      continue
    fi

    exe_path="$(readlink -f "$proc_dir/exe" 2>/dev/null || true)"
    [[ -n "$exe_path" ]] || continue
    [[ "$exe_path" == "$binary_path" ]] || continue
    matched+=("$pid")
  done

  if (( ${#matched[@]} == 0 )); then
    return
  fi

  for pid in "${matched[@]}"; do
    kill -KILL "$pid" 2>/dev/null || true
  done

  local deadline=$((SECONDS + 5))
  while (( SECONDS < deadline )); do
    local remaining=()
    for pid in "${matched[@]}"; do
      if kill -0 "$pid" 2>/dev/null; then
        remaining+=("$pid")
      fi
    done

    if (( ${#remaining[@]} == 0 )); then
      return
    fi
    sleep 0.15
    matched=("${remaining[@]}")
  done

  printf 'stale rmu-mcp-server processes are still running for %s (pids: %s)\n' \
    "$binary_path" \
    "$(IFS=,; printf '%s' "${matched[*]}")" \
    >&2
  exit 1
}

build_release_if_needed() {
  if ! rebuild_required; then
    return
  fi

  (
    cd "$repo_root"
    cargo build --release -p rmu-mcp-server
  )

  if [[ ! -x "$binary_path" ]]; then
    printf 'rmu-mcp-server not found at %s after rebuild\n' "$binary_path" >&2
    exit 1
  fi
}

stop_stale_server_processes
build_release_if_needed

exec "$binary_path" "$@"
