#!/usr/bin/env bash
set -euo pipefail

script_path="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_path/.." && pwd)"
codex_bin_dir="${HOME}/.codex/bin"
installed_binary="$codex_bin_dir/rmu-mcp-server"
backup_binary="$codex_bin_dir/rmu-mcp-server.previous"

running_binary_pids() {
  local binary_path="$1"
  if ! command -v pgrep >/dev/null 2>&1; then
    return
  fi
  while IFS= read -r pid; do
    exe_path="$(readlink "/proc/$pid/exe" 2>/dev/null || true)"
    [[ "$exe_path" == "$binary_path" ]] || continue
    printf '%s\n' "$pid"
  done < <(pgrep -f 'rmu-mcp-server' || true)
}

stop_source_binary_processes() {
  local source_binary="$1"
  while IFS= read -r pid; do
    kill -9 "$pid" 2>/dev/null || true
  done < <(running_binary_pids "$source_binary")
}

source_binary=""
installed_profile=""
for profile in release debug; do
  if [[ "$profile" == "release" ]]; then
    candidate="$repo_root/target/release/rmu-mcp-server"
    stop_source_binary_processes "$candidate"
    (
      cd "$repo_root"
      cargo build --release -p rmu-mcp-server
    ) || continue
  else
    candidate="$repo_root/target/debug/rmu-mcp-server"
    stop_source_binary_processes "$candidate"
    (
      cd "$repo_root"
      cargo build -p rmu-mcp-server
    ) || continue
  fi

  if [[ -x "$candidate" ]]; then
    source_binary="$candidate"
    installed_profile="$profile"
    break
  fi
done

if [[ -z "$source_binary" ]]; then
  printf 'failed to build rmu-mcp-server for both release and debug profiles\n' >&2
  exit 1
fi

if [[ ! -x "$source_binary" ]]; then
  printf 'release binary not found at %s\n' "$source_binary" >&2
  exit 1
fi

mkdir -p "$codex_bin_dir"
mapfile -t running_installed_pids < <(running_binary_pids "$installed_binary")
if [[ "${#running_installed_pids[@]}" -gt 0 ]]; then
  if [[ -f "$installed_binary" ]]; then
    cp -f "$installed_binary" "$backup_binary"
  fi
  printf 'pending_restart=true\n'
  printf 'restart_hint=restart the Codex app, then rerun this installer; opening a new chat is not enough because MCP transport is app-global\n'
  printf 'installed_profile=%s\n' "$installed_profile"
  printf 'installed_binary=%s\n' "$installed_binary"
  printf 'backup_binary=%s\n' "$backup_binary"
  printf 'repo_root=%s\n' "$repo_root"
  printf 'running_installed_pids=%s\n' "$(IFS=,; echo "${running_installed_pids[*]}")"
  printf 'active Codex RMU server is running; installed binary was not replaced\n' >&2
  exit 0
fi

if [[ -f "$installed_binary" ]]; then
  cp -f "$installed_binary" "$backup_binary"
fi
cp -f "$source_binary" "$installed_binary"

printf 'pending_restart=false\n'
printf 'installed_profile=%s\n' "$installed_profile"
printf 'installed_binary=%s\n' "$installed_binary"
printf 'backup_binary=%s\n' "$backup_binary"
printf 'repo_root=%s\n' "$repo_root"
