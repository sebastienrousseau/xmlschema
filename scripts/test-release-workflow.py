#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0 OR MIT
"""Validate dependency ordering and failure propagation without publishing."""
from pathlib import Path
import re
import tomllib
import unittest

ROOT = Path(__file__).resolve().parent.parent
FAMILY = {"oxml", "xmlschema", "oxml-json"}


class ReleaseWorkflow(unittest.TestCase):
    def setUp(self):
        self.workflow = (ROOT / ".github/workflows/release.yml").read_text()
        self.manifest = tomllib.loads((ROOT / "Cargo.toml").read_text())

    def test_prerequisites_precede_compilation(self):
        dependencies = FAMILY & self.manifest.get("dependencies", {}).keys()
        wait = "- name: Wait for the crates this one depends on"
        if not dependencies:
            self.assertNotIn(wait, self.workflow)
            return
        self.assertIn(wait, self.workflow)
        self.assertLess(self.workflow.index(wait), self.workflow.index("run: cargo test"))
        block = self.workflow.split(wait, 1)[1].split("\n      - ", 1)[0]
        match = re.search(r"for dep in ([^;]+); do", block)
        self.assertIsNotNone(match)
        self.assertEqual(set(match[1].split()), dependencies)

    def test_publication_failures_are_not_ignored(self):
        self.assertNotIn("continue-on-error: true", self.workflow)
        self.assertNotIn("publish with scripts/publish.sh", self.workflow)

    def test_shared_actions_match_ci_workflow_pins(self):
        ci_workflow = (ROOT / ".github/workflows/ci.yml").read_text()
        ci_pins = dict(re.findall(r"uses:\s+([A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+)@([0-9a-f]{40})", ci_workflow))
        for action, sha in re.findall(r"uses:\s+([A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+)@([0-9a-f]{40})", self.workflow):
            if action in ci_pins:
                self.assertEqual(sha, ci_pins[action], f"{action} pin in release.yml differs from ci.yml")


if __name__ == "__main__":
    unittest.main()
