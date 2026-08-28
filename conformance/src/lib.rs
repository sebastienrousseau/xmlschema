// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 xmlschema. All rights reserved.

//! The W3C XML Schema Test Suite, run against `xmlschema`.
//!
//! `xmlschema` implements a subset of XSD 1.0 and skips what it does
//! not understand. That makes a naive pass rate meaningless: a schema
//! whose constraints were all skipped accepts every document, so every
//! test expecting "valid" agrees with it, and counting those as passes
//! reports enforcement that never happened.
//!
//! So an outcome is only [`Outcome::Pass`] when the schema is enforced
//! **in full** — [`xmlschema::support::unsupported`] returns nothing —
//! *and* the answer matches. A schema with any skipped construct is
//! [`Outcome::Unsupported`] whatever the answer, including when the
//! answer happens to be right. Those would-be passes are counted
//! separately and published, because their size is the whole argument
//! for measuring this way.

pub mod baseline;
pub mod catalog;
pub mod outcome;
pub mod runner;
pub mod sha256;

use std::path::PathBuf;

/// The suite release this harness is pinned to.
pub const SUITE_RELEASE: &str = "xsts-2007-06-20";

/// Where it comes from.
pub const SUITE_URL: &str = "https://www.w3.org/XML/2004/xml-schema-test-suite/xmlschema2006-11-06/xsts-2007-06-20.tar.gz";

/// SHA-256 of the tarball, verified on download.
///
/// Pinned so the denominator cannot move underneath a published
/// figure, and so a truncated or challenge-page download fails loudly
/// instead of yielding an empty directory and a green run over zero
/// tests.
pub const SUITE_SHA256: &str =
    "902176b25e4111cf96b08663107521a4992e8ea67aad6b815592a6a5b4b9ea06";

/// Tests in the suite: 14,328 schema tests and 25,092 instance tests.
pub const REAL_TESTS: usize = 39_420;

/// The extracted suite, if it has been downloaded.
#[must_use]
pub fn data_dir() -> Option<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("xmlschema2006-11-06");
    root.join("suite.xml").is_file().then_some(root)
}

/// Skip a test when the suite is absent, rather than failing.
///
/// A normal `cargo test` must not need a network.
#[macro_export]
macro_rules! require_suite {
    () => {
        match $crate::data_dir() {
            Some(root) => root,
            // Set `XMLSCHEMA_REQUIRE_SUITE=1` to make a missing suite
            // a failure instead of a skip.
            //
            // `cargo test` on a fresh clone has no network and should
            // not need one, so the default is to skip. But a skipped
            // test is reported as a *passing* test, so a gate that
            // accepts the skip prints success having run none of the
            // 39,420 tests. `oxml` shipped exactly that for a while;
            // CI here sets the variable so it cannot happen.
            None if ::std::env::var_os("XMLSCHEMA_REQUIRE_SUITE").is_none() => {
                eprintln!(
                    "the suite is not downloaded; run\n  \
                     cargo run -p xmlschema-conformance --bin download"
                );
                return;
            }
            None => panic!(
                "XMLSCHEMA_REQUIRE_SUITE is set and the suite is not \
                 present. Run `cargo run -p xmlschema-conformance \
                 --bin download`. Refusing to report a pass for tests \
                 that did not run."
            ),
        }
    };
}
