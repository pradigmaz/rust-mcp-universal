#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
repo_root="$(cd -- "$script_dir/.." && pwd -P)"
release_binary_path="$repo_root/target/release/rmu-mcp-server"
debug_binary_path="$repo_root/target/debug/rmu-mcp-server"

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
  local binary_path="$1"
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
  local binary_path="$1"
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

build_profile_if_needed() {
  local binary_path="$1"
  shift

  if ! rebuild_required "$binary_path"; then
    return 0
  fi

  (
    cd "$repo_root"
    cargo "$@"
  ) || return 1

  if [[ ! -x "$binary_path" ]]; then
    return 1
  fi
  return 0
}

run_binary_path=""

stop_stale_server_processes "$release_binary_path"
if build_profile_if_needed "$release_binary_path" build --release -p rmu-mcp-server; then
  run_binary_path="$release_binary_path"
else
  stop_stale_server_processes "$debug_binary_path"
  if build_profile_if_needed "$debug_binary_path" build -p rmu-mcp-server; then
    run_binary_path="$debug_binary_path"
  fi
fi

if [[ -z "$run_binary_path" ]]; then
  printf 'failed to prepare fresh rmu-mcp-server from both release and debug profiles\n' >&2
  exit 1
fi

exec "$run_binary_path" "$@"
