// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 xmlschema. All rights reserved.

//! The test count the docs quote must be the number of tests there are.
//!
//! Both `README.md` and `doc/TESTING.md` open by stating how many
//! tests this crate has. Nothing checked it, and by 0.0.7 both said
//! 207 while the suite had grown to 235 -- the same drift the
//! conformance figures suffered, from the same cause: a measured
//! number restated in prose that no run reads back.

use std::fs;
use std::path::Path;

/// Every test declared in `tests/`, this file's own included.
///
/// Counted as whole lines rather than as a substring. A substring
/// search finds the attribute, but it also finds every mention of it
/// in prose and in this function's own source -- the first version
/// counted itself three times and demanded a number three too high.
fn declared_tests(dir: &Path) -> usize {
    let mut total = 0;
    let entries = fs::read_dir(dir).expect("tests/ is readable");
    for entry in entries {
        let path = entry.expect("a directory entry").path();
        if path.extension().is_some_and(|e| e == "rs") {
            let text = fs::read_to_string(&path).expect("a readable test");
            total += text.lines().filter(|l| l.trim() == "#[test]").count();
        }
    }
    total
}

#[test]
fn the_published_test_count_is_current() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let count = declared_tests(&dir);
    let stated = format!("{count} tests");
    for name in ["README.md", "doc/TESTING.md"] {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(name);
        let text = fs::read_to_string(&path).expect("a readable document");
        assert!(
            text.contains(&stated),
            "{name} does not say `{stated}`; \
             tests/ declares {count} tests"
        );
    }
}
