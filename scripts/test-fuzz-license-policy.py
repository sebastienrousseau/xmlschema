#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0 OR MIT
"""Keep the approved fuzz license exception out of runtime policy."""
from pathlib import Path
import re
import tomllib
import unittest

ROOT = Path(__file__).resolve().parent.parent


class FuzzLicensePolicy(unittest.TestCase):
    def test_runtime_policy_does_not_allow_ncsa(self):
        policy = tomllib.loads((ROOT / "deny.toml").read_text())
        self.assertNotIn("NCSA", policy["licenses"]["allow"])
        self.assertFalse(policy["licenses"].get("exceptions"))

    def test_fuzz_exception_is_exact(self):
        path = ROOT / "fuzz/deny.exceptions.toml"
        self.assertTrue(path.is_file(), "missing scoped fuzz license exception")
        self.assertEqual(tomllib.loads(path.read_text()), {
            "exceptions": [{"crate": "libfuzzer-sys@0.4.13", "allow": ["NCSA"]}]
        })
        lock = tomllib.loads((ROOT / "fuzz/Cargo.lock").read_text())
        versions = {p["version"] for p in lock["package"] if p["name"] == "libfuzzer-sys"}
        self.assertEqual(versions, {"0.4.13"})

    def test_ci_checks_both_graphs(self):
        workflow = (ROOT / ".github/workflows/security.yml").read_text()
        # Both graphs run the same SHA-pinned action; the pin itself moves
        # with Dependabot, so assert its shape rather than one revision.
        pins = re.findall(
            r"uses: EmbarkStudios/cargo-deny-action@([0-9a-f]{40})\b", workflow
        )
        self.assertEqual(len(pins), 2)
        self.assertEqual(len(set(pins)), 1)
        self.assertIn("manifest-path: fuzz/Cargo.toml", workflow)
        self.assertIn("arguments: --all-features --locked", workflow)


if __name__ == "__main__":
    unittest.main()
