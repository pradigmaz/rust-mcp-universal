#!/usr/bin/env python3
from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path

WINDOWS_HOME_RE = re.compile(r"(?i)\b[A-Z]:\\Users\\([^\\\s]+)\\")
UNIX_HOME_RE = re.compile(r"(?<![\w$])/(Users|home)/([^/\s]+)(?=/)")
WINDOWS_ALLOWED = {"<you>", "%userprofile%", "%username%"}
UNIX_ALLOWED = {"<you>", "$home", "${home}"}
TEXT_SUFFIXES = {".md", ".json", ".toml"}
INCLUDED_PREFIXES = (".github", "schemas", "baseline")
EXCLUDED_PREFIXES = (
    ".git",
    "target",
    ".rmu",
    ".codex",
    ".codex-planning",
    ".tmp_integration",
    ".tmp_scope_check",
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Fail on real user-home paths in tracked docs/examples/config artifacts."
    )
    parser.add_argument("paths", nargs="*", help="Optional explicit files to scan.")
    parser.add_argument("--root", default=".", help="Repository root to scan.")
    return parser.parse_args()


def is_candidate_path(path: Path) -> bool:
    parts = path.parts
    if any(part.startswith(".tmp") for part in parts):
        return False
    if parts and parts[0] in EXCLUDED_PREFIXES:
        return False
    if len(parts) == 1:
        return path.suffix.lower() in TEXT_SUFFIXES
    return parts[0] in INCLUDED_PREFIXES


def git_ls_files(root: Path) -> list[Path]:
    result = subprocess.run(
        ["git", "ls-files"],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    )
    files: list[Path] = []
    for raw in result.stdout.splitlines():
        raw = raw.strip()
        if not raw:
            continue
        path = Path(raw)
        if is_candidate_path(path):
            files.append(root / path)
    return files


def find_violations(text: str) -> list[str]:
    violations: list[str] = []
    for match in WINDOWS_HOME_RE.finditer(text):
        user = match.group(1).lower()
        if user not in WINDOWS_ALLOWED:
            violations.append(match.group(0))
    for match in UNIX_HOME_RE.finditer(text):
        user = match.group(2).lower()
        if user not in UNIX_ALLOWED:
            violations.append(match.group(0))
    return violations


def load_candidate_files(root: Path, explicit_paths: list[str]) -> list[Path]:
    if explicit_paths:
        return [Path(path) if Path(path).is_absolute() else root / path for path in explicit_paths]
    return git_ls_files(root)


def main() -> int:
    args = parse_args()
    root = Path(args.root).resolve()
    failures: list[str] = []

    for path in load_candidate_files(root, args.paths):
        if not path.is_file():
            continue
        try:
            text = path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            continue
        violations = find_violations(text)
        if violations:
            relative = path.relative_to(root) if path.is_relative_to(root) else path
            sample = ", ".join(sorted(set(violations))[:3])
            failures.append(f"{relative}: {sample}")

    if failures:
        print("Privacy guard failed. Real user-home paths found:", file=sys.stderr)
        for failure in failures:
            print(f" - {failure}", file=sys.stderr)
        return 1

    print("Privacy guard passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
