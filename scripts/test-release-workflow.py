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


if __name__ == "__main__":
    unittest.main()
