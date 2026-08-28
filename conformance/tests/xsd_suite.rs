// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (c) 2026 xmlschema. All rights reserved.

//! The W3C XML Schema Test Suite, ratcheted against a baseline.
//!
//! Run `cargo run -p xmlschema-conformance --bin download` first.
//! Without the suite these tests skip rather than fail, so a normal
//! `cargo test` does not need a network. CI sets
//! `XMLSCHEMA_REQUIRE_SUITE=1` so that a skip cannot be mistaken for a
//! pass there.
//!
//! Regenerate the baseline deliberately, never automatically:
//!
//! ```text
//! XMLSCHEMA_UPDATE_BASELINE=1 cargo test -p xmlschema-conformance
//! ```
//!
//! Before this file existed, the 95.6% figure in the README was
//! produced by running a binary by hand. Nothing compared one run to
//! the next, and nothing ran the suite in CI.

use std::path::Path;

use xmlschema_conformance::{
    REAL_TESTS, baseline, catalog, outcome::Outcome, require_suite, runner,
};

fn baseline_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("baselines")
        .join("w3c-xsd.tsv")
}

#[test]
fn the_suite_matches_its_baseline() {
    let root = require_suite!();
    let cases = catalog::load(&root).expect("catalog loads");

    // Test-count drift means the suite on disk is not the release
    // pinned in `SUITE_SHA256`, and every rate computed from it would
    // be against a different denominator.
    assert_eq!(
        cases.len(),
        REAL_TESTS,
        "test count drift: expected {REAL_TESTS}, found {}",
        cases.len()
    );

    let (results, counts) = runner::run_all(&cases);
    let rendered = baseline::render(&results, &counts);

    let path = baseline_path();
    if std::env::var_os("XMLSCHEMA_UPDATE_BASELINE").is_some() {
        std::fs::create_dir_all(path.parent().expect("has a parent"))
            .expect("create baselines dir");
        std::fs::write(&path, &rendered).expect("write baseline");
        eprintln!("baseline updated: {counts}");
        return;
    }

    let previous = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "no baseline at {}; generate one with \
             XMLSCHEMA_UPDATE_BASELINE=1 cargo test -p \
             xmlschema-conformance",
            path.display()
        )
    });

    let differences = baseline::diff(
        &baseline::parse(&previous),
        &baseline::parse(&rendered),
    );
    assert!(
        differences.is_empty(),
        "conformance changed against the baseline.\n\n{}\n\n\
         If this is an intended improvement, regenerate with:\n  \
         XMLSCHEMA_UPDATE_BASELINE=1 cargo test -p \
         xmlschema-conformance\n\n\
         An *improvement* fails here too, on purpose: a pass rate that \
         drifts upward because tests started being skipped rather than \
         passing is indistinguishable from real progress unless every \
         change is reviewed.",
        differences
            .iter()
            .take(40)
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// A panic is never acceptable, at any baseline.
///
/// Separate from the ratchet on purpose: a caller cannot catch one,
/// and a schema is an input they did not write.
#[test]
fn no_schema_in_the_suite_panics_the_validator() {
    let root = require_suite!();
    let cases = catalog::load(&root).expect("catalog loads");
    let (results, _) = runner::run_all(&cases);
    let panics: Vec<&str> = results
        .iter()
        .filter(|r| r.outcome == Outcome::Panic)
        .map(|r| r.id.as_str())
        .collect();
    assert!(panics.is_empty(), "these schemas panicked: {panics:?}");
}

/// The figures published in the README must be the ones the suite
/// produces.
///
/// Prose restating a measured number drifts from it silently. In
/// `oxml` a per-submission table sat two eras out of date while the
/// summary above it was current, in the same file, for months.
#[test]
fn the_published_figures_are_current() {
    let root = require_suite!();
    let cases = catalog::load(&root).expect("catalog loads");
    let (_, counts) = runner::run_all(&cases);

    let readme = include_str!("../../README.md");
    let rate = format!("{:.1}%", counts.pass_rate());
    assert!(
        readme.contains(&rate),
        "README.md does not state the current pass rate `{rate}`"
    );
    let total = format!("{}", counts.total());
    let pretty = format!(
        "{},{}",
        &total[..total.len() - 3],
        &total[total.len() - 3..]
    );
    assert!(
        readme.contains(&pretty) || readme.contains(&total),
        "README.md does not state the suite size `{pretty}`"
    );
}
