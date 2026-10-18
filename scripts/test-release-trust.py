#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0 OR MIT
"""Exercise release trust boundaries without credentials or publication."""
import os
from pathlib import Path
import re
import subprocess
import tempfile
import textwrap
import tomllib
import unittest

ROOT = Path(__file__).resolve().parent.parent
SIGNER = 'sebastian.rousseau@gmail.com namespaces="git" ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIDjGRQyeM79nAb4GHcpGzXMKq7HE3xyEOev4gVsnBN1e\n'


def run(args, cwd, env):
    return subprocess.run(args, cwd=cwd, env=env, check=True,
                          text=True, capture_output=True, timeout=30)


class ReleaseTrust(unittest.TestCase):
    def setUp(self):
        self.workflow = (ROOT / ".github/workflows/release.yml").read_text()
        self.crate = tomllib.loads((ROOT / "Cargo.toml").read_text())["package"]["name"]

    def test_oidc_is_the_only_publication_credential(self):
        self.assertNotIn("secrets.CARGO_REGISTRY_TOKEN", self.workflow)
        self.assertIn("id-token: write", self.workflow)
        self.assertIn("contents: read", self.workflow)
        self.assertIn("uses: rust-lang/crates-io-auth-action@c6f97d42243bad5fab37ca0427f495c86d5b1a18", self.workflow)
        self.assertIn("CARGO_REGISTRY_TOKEN: ${{ steps.auth.outputs.token }}", self.workflow)
        auth = self.workflow.index("id: auth")
        self.assertLess(self.workflow.index("run: cargo test --locked"), auth)
        self.assertLess(auth, self.workflow.index("run: cargo publish --locked"))
        self.assertIn("fetch-depth: 0", self.workflow)

    def test_approved_public_signer(self):
        path = ROOT / ".github/allowed_signers"
        self.assertTrue(path.is_file(), "missing approved public signer")
        self.assertEqual(path.read_text(), SIGNER)

    def test_real_signed_tag_gate(self):
        marker = "- name: The tag must be signed by a listed key"
        self.assertIn(marker, self.workflow)
        self.assertLess(self.workflow.index(marker), self.workflow.index("id: published"))
        block = self.workflow.split(marker, 1)[1].split("\n      - ", 1)[0]
        script = textwrap.dedent(block.split("run: |\n", 1)[1])
        env = dict(os.environ, GIT_CONFIG_NOSYSTEM="1",
                   GIT_CONFIG_GLOBAL=os.devnull, GITHUB_REF_NAME="v0.0.11")
        with tempfile.TemporaryDirectory() as folder:
            fixture = Path(folder)
            run(["git", "init", "-q", "."], fixture, env)
            for key in ("trusted", "untrusted"):
                run(["ssh-keygen", "-q", "-t", "ed25519", "-N", "",
                     "-f", str(fixture / key)], fixture, env)
            settings = {"user.name": "Release test", "user.email": "test@example.invalid",
                        "gpg.format": "ssh", "commit.gpgsign": "false",
                        "tag.gpgsign": "false", "core.hooksPath": os.devnull,
                        "user.signingkey": str(fixture / "trusted")}
            for name, value in settings.items():
                run(["git", "config", name, value], fixture, env)
            run(["git", "commit", "-q", "--allow-empty", "-m", "fixture"], fixture, env)
            (fixture / ".github").mkdir()
            public = (fixture / "trusted.pub").read_text().strip()
            (fixture / ".github/allowed_signers").write_text(
                'test@example.invalid namespaces="git" ' + public + "\n")
            cases = [
                (["-s", "-m", self.crate + " v0.0.11"], True),
                (["-s", "-m", "wrong v0.0.11"], False),
                (["-a", "-m", self.crate + " v0.0.11"], False),
                ([], False),
                (["-s", "-u", str(fixture / "untrusted"),
                  "-m", self.crate + " v0.0.11"], False),
            ]
            for args, accepted in cases:
                with self.subTest(args=args):
                    run(["git", "tag", *args, "v0.0.11"], fixture, env)
                    result = subprocess.run(["bash", "-euo", "pipefail", "-c", script],
                                            cwd=fixture, env=env, text=True,
                                            capture_output=True, timeout=30)
                    self.assertEqual(result.returncode == 0, accepted, result.stderr)
                    run(["git", "tag", "-d", "v0.0.11"], fixture, env)


if __name__ == "__main__":
    unittest.main()
