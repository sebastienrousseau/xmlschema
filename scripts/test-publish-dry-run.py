#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0 OR MIT
"""Run the laptop release helper with isolated tools; never contact a registry."""

import os
from pathlib import Path
import subprocess
import tempfile
import unittest


GIT_STUB = """#!/bin/sh
case "$*" in
  'rev-parse --abbrev-ref HEAD') echo "${TEST_BRANCH:-main}" ;;
  'fetch origin --quiet') exit 0 ;;
  'rev-parse HEAD') echo reviewed ;;
  'rev-parse origin/main') echo "${TEST_REMOTE_HEAD:-reviewed}" ;;
  'status --porcelain') printf '%s' "${TEST_DIRTY:-}" ;;
  *) exit 64 ;;
esac
"""
CARGO_STUB = """#!/bin/sh
printf '%s\\n' "$*" >> "$TEST_CALLS"
case "$1" in
  metadata) echo '{"packages":[{"name":"xmlschema","version":"0.0.13"}]}' ;;
  publish)
    case " $* " in
      *' --dry-run '*) exit "${TEST_DRY_RUN_EXIT:-0}" ;;
      *) echo 'unexpected registry upload' >&2; exit 42 ;;
    esac ;;
  *) exit 64 ;;
esac
"""
GATE_STUB = """#!/bin/sh
echo gate >> "$TEST_CALLS"
exit "${TEST_GATE_EXIT:-0}"
"""


def executable(path: Path, source: str) -> None:
    path.write_text(source)
    path.chmod(0o755)


def run_helper(**settings: str) -> tuple[subprocess.CompletedProcess, list[str]]:
    source = Path(__file__).resolve().parent / "publish.sh"
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        scripts = root / "scripts"
        scripts.mkdir()
        executable(scripts / "publish.sh", source.read_text())
        executable(scripts / "gate.sh", GATE_STUB)
        binaries = root / "bin"
        binaries.mkdir()
        executable(binaries / "git", GIT_STUB)
        executable(binaries / "cargo", CARGO_STUB)
        executable(binaries / "curl", "#!/bin/sh\nexit 42\n")
        calls = root / "calls"
        result = subprocess.run(
            ["bash", str(scripts / "publish.sh")],
            cwd=root.parent,
            env={**os.environ, **settings, "TEST_CALLS": str(calls),
                 "PATH": str(binaries) + os.pathsep + os.environ["PATH"]},
            input="y\n", capture_output=True, text=True, timeout=10, check=False,
        )
        return result, calls.read_text().splitlines()


class PublishDryRun(unittest.TestCase):
    def test_confirmation_cannot_trigger_an_upload(self) -> None:
        result, calls = run_helper()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(calls, ["metadata --no-deps --format-version 1 --locked",
                                "gate", "publish --dry-run --locked"])

    def test_failed_gate_or_dry_run_is_propagated(self) -> None:
        for setting in ["TEST_GATE_EXIT", "TEST_DRY_RUN_EXIT"]:
            with self.subTest(setting=setting):
                result, calls = run_helper(**{setting: "23"})
                self.assertEqual(result.returncode, 23, result.stderr)
                if setting == "TEST_GATE_EXIT":
                    self.assertFalse(any(call.startswith("publish") for call in calls))

    def test_unreviewed_tree_stops_before_gate_and_package(self) -> None:
        for setting, value in [("TEST_BRANCH", "feature"),
                               ("TEST_REMOTE_HEAD", "new-main"),
                               ("TEST_DIRTY", " M Cargo.toml")]:
            with self.subTest(setting=setting):
                result, calls = run_helper(**{setting: value})
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(len(calls), 1, calls)


if __name__ == "__main__":
    unittest.main()
