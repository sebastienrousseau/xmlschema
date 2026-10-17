#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0 OR MIT
"""Prove example coverage belongs to this package, not a sibling file."""

import contextlib
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parent.parent
CORE = (ROOT / "crates/oxml/src").is_dir()


class ExampleCoverageContract(unittest.TestCase):
    def test_coverage_identity_and_missing_code(self):
        spec = importlib.util.spec_from_file_location(
            "example_checker", ROOT / "scripts/check-example-coverage.py"
        )
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        modes = [
            "covered", "relative-covered", "uncovered", "generic",
            "no-file", "shadowed", "missing-own", "shadowed-covered",
        ]
        for mode in modes:
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                src = root / ("crates/oxml/src" if CORE else "src")
                src.mkdir(parents=True)
                source = src / "lib.rs"
                declaration = (
                    "pub fn probe<T>() {}" if mode == "generic"
                    else "pub fn probe() {}"
                )
                source.write_text(declaration + "\n")
                covered = mode in ["covered", "relative-covered", "shadowed-covered"]
                segments = [] if mode == "generic" else [
                    [1, 1, int(covered), True, True, False]
                ]
                filename = (
                    str(source.relative_to(root)) if mode == "relative-covered"
                    else str(source)
                )
                files = [] if mode in ["no-file", "missing-own"] else [
                    {"filename": filename, "segments": segments}
                ]
                if mode in ["shadowed", "missing-own", "shadowed-covered"]:
                    files.insert(0, {
                        "filename": str(root / "dependency/src/lib.rs"),
                        "segments": [[1, 1, int(not covered), True, True, False]],
                    })
                report = json.dumps({"data": [{"files": files}]})
                names = [("probe", [])] if CORE else ["probe"]
                with (
                    patch.object(module, "ROOT", root),
                    patch.object(module, "SRC", src),
                    patch.object(module, "examples", return_value=names),
                    patch.object(module, "run", return_value=report),
                    contextlib.redirect_stdout(io.StringIO()),
                ):
                    self.assertEqual(module.main(), 0 if covered else 1)


if __name__ == "__main__":
    unittest.main()
