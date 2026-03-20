from __future__ import annotations

import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("check_privacy_paths.py")


class PrivacyGuardTests(unittest.TestCase):
    def run_guard(self, *paths: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(SCRIPT), "--root", ".", *paths],
            check=False,
            capture_output=True,
            text=True,
            cwd=SCRIPT.parent.parent,
        )

    def test_rejects_real_user_home_path(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "failing.md"
            path.write_text(r"Bad path: C:\Users\RealUser\.codex\config.toml", encoding="utf-8")
            result = self.run_guard(str(path))

        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn("Privacy guard failed", result.stderr)
        self.assertIn("C:\\Users\\RealUser\\", result.stderr)

    def test_allows_placeholders_and_env_vars(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "passing.md"
            path.write_text(
                "\n".join(
                    [
                        r"Placeholder: C:\Users\<you>\.codex\config.toml",
                        "Unix env: $HOME/.config/tool",
                        r"Windows env: %USERPROFILE%\tool\config.toml",
                    ]
                ),
                encoding="utf-8",
            )
            result = self.run_guard(str(path))

        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("Privacy guard passed.", result.stdout)


if __name__ == "__main__":
    unittest.main()
