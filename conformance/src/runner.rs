// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 xmlschema. All rights reserved.

//! Running the suite, and deciding what each answer is worth.
//!
//! The rule that makes the number honest: **a test is a pass only when
//! the schema is enforced in full.** `xmlschema` skips constructs it
//! does not understand, and a schema whose constraints were skipped
//! accepts every document — so an agreement with the suite proves
//! nothing about enforcement. Those tests are `Unsupported`, and the
//! ones whose answer happened to agree are counted separately as
//! *vacuous* so the size of the flattery avoided is published rather
//! than asserted.

use std::panic::{AssertUnwindSafe, catch_unwind};

use crate::catalog::{Case, Expected};
use crate::outcome::{Counts, Outcome};

/// The suite's own files, as a schema source.
///
/// A `schemaLocation` in the suite is a path relative to the schema
/// that names it, and the files are on disk beside it. Supplying them
/// is the harness's job, not the library's: `xmlschema` performs no
/// I/O, and a conformance runner that could not resolve an import
/// would report every schema using one as unenforceable -- measuring
/// the harness's limits rather than the crate's.
struct SuiteFiles {
    /// Documents already read, by resolved path.
    loaded: std::collections::BTreeMap<String, String>,
}

impl SuiteFiles {
    /// Read every document reachable from `schema`, following
    /// `schemaLocation` relative to whichever file names it.
    fn gather(schema: &std::path::Path) -> Self {
        let mut loaded = std::collections::BTreeMap::new();
        let mut queue = vec![schema.to_path_buf()];
        // Bounded: a pair of schemas may reference one another, and
        // the suite contains some that do.
        for _ in 0..64 {
            let Some(path) = queue.pop() else {
                break;
            };
            let key = path.to_string_lossy().into_owned();
            if loaded.contains_key(&key) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            for location in schema_locations(&text) {
                let mut next = path.clone();
                let _ = next.pop();
                for part in location.split('/') {
                    match part {
                        "." | "" => {}
                        ".." => {
                            let _ = next.pop();
                        }
                        other => next.push(other),
                    }
                }
                queue.push(next);
            }
            let _ = loaded.insert(key, text);
        }
        Self { loaded }
    }
}

/// Every `schemaLocation` in a schema document, read textually.
///
/// Textually because the document may not parse -- the suite is full
/// of schemas that do not -- and a source that only worked for valid
/// input would resolve nothing for exactly the tests that need it.
fn schema_locations(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find("schemaLocation") {
        rest = &rest[at + "schemaLocation".len()..];
        let Some(quote) = rest.find(['"', '\'']) else {
            break;
        };
        let delimiter = rest.as_bytes()[quote] as char;
        rest = &rest[quote + 1..];
        let Some(end) = rest.find(delimiter) else {
            break;
        };
        out.push(rest[..end].to_owned());
        rest = &rest[end + 1..];
    }
    out
}

impl xmlschema::SchemaSource for SuiteFiles {
    fn fetch(&self, location: &str) -> Option<&str> {
        // The location is relative to the file that named it, and the
        // gather pass resolved each one already -- so match on the
        // tail rather than re-resolving without knowing the base.
        self.loaded
            .iter()
            .find(|(path, _)| path.ends_with(location.trim_start_matches("./")))
            .map(|(_, text)| text.as_str())
    }
}

/// One test's result, with enough context to report it.
#[derive(Debug, Clone)]
pub struct Record {
    /// The test's name, qualified by its group.
    pub id: String,
    /// Which test set it came from.
    pub set: String,
    /// What happened.
    pub outcome: Outcome,
    /// For an unsupported test, whether the answer would have agreed.
    pub would_have_matched: bool,
    /// The first construct that stopped the schema being enforced.
    pub reason: Option<String>,
    /// Which way a failure went, which decides how bad it is.
    pub direction: Option<Direction>,
    /// The first violation reported, for grouping failures by cause.
    pub detail: Option<String>,
}

/// Which way a disagreement went.
///
/// The two are not equally bad. Rejecting a document the suite calls
/// valid breaks a caller whose schema is correct; accepting one the
/// suite calls invalid is a check that is missing. The first is the
/// one to fix first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Expected valid, reported invalid -- over-strict.
    WronglyRejected,
    /// Expected invalid, reported valid -- under-strict.
    WronglyAccepted,
}

/// Run every case.
#[must_use]
pub fn run_all(cases: &[Case]) -> (Vec<Record>, Counts) {
    let mut records = Vec::with_capacity(cases.len());
    let mut counts = Counts::default();
    for case in cases {
        let record = run_one(case);
        counts.add(record.outcome, record.would_have_matched);
        records.push(record);
    }
    (records, counts)
}

/// Run one case.
#[must_use]
pub fn run_one(case: &Case) -> Record {
    let id = format!("{}/{}", case.group, case.name);
    let mut record = Record {
        id,
        set: case.set.clone(),
        outcome: Outcome::Blocked,
        would_have_matched: false,
        reason: None,
        direction: None,
        detail: None,
    };

    // A panic is a result, not an abort: the point of running forty
    // thousand hostile documents is to find one.
    let outcome = catch_unwind(AssertUnwindSafe(|| decide(case)));
    match outcome {
        Ok(decided) => {
            record.outcome = decided.outcome;
            record.would_have_matched = decided.matched;
            record.reason = decided.reason;
            record.direction = decided.direction;
            record.detail = decided.detail;
        }
        Err(_) => record.outcome = Outcome::Panic,
    }
    record
}

/// Everything `decide` works out about one case.
struct Decided {
    outcome: Outcome,
    matched: bool,
    reason: Option<String>,
    direction: Option<Direction>,
    detail: Option<String>,
}

impl Decided {
    fn new(outcome: Outcome, matched: bool) -> Self {
        Self {
            outcome,
            matched,
            reason: None,
            direction: None,
            detail: None,
        }
    }

    fn because(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }
}

/// Classify an answer that did not match, and say why.
fn judged(
    matched: bool,
    expected_valid: bool,
    detail: Option<String>,
) -> Decided {
    let mut d = Decided::new(
        if matched {
            Outcome::Pass
        } else {
            Outcome::Fail
        },
        matched,
    );
    if !matched {
        d.direction = Some(if expected_valid {
            Direction::WronglyRejected
        } else {
            Direction::WronglyAccepted
        });
        d.detail = detail;
    }
    d
}

/// Decide one case.
fn decide(case: &Case) -> Decided {
    let Some(schema_path) = case.schema.as_ref() else {
        return Decided::new(Outcome::Blocked, false)
            .because("the group names no schema document");
    };
    let Ok(schema_src) = std::fs::read_to_string(schema_path) else {
        return Decided::new(Outcome::Blocked, false)
            .because("schema unreadable");
    };

    let expected_valid = case.expected == Expected::Valid;

    // Whether the schema is enforced in full is decided from the
    // schema document itself, independently of whether it parses.
    let gaps = match oxml::parse(&schema_src) {
        Ok(doc) => xmlschema::support::unsupported(&doc),
        // A schema that is not well-formed XML is a legitimate
        // "invalid schema" answer, not a gap in enforcement.
        Err(_) => Vec::new(),
    };
    let reason = gaps
        .first()
        .map(|u| format!("{}: {}", u.construct, u.effect));

    // Gathered only when the schema actually references something.
    // Reading the neighbourhood of every schema in a 39,420-test
    // suite -- almost none of which import anything -- turned a
    // seconds-long run into one that did not finish.
    let parsed = if schema_src.contains("schemaLocation") {
        let files = SuiteFiles::gather(schema_path);
        xmlschema::parse_schema_with(&schema_src, &files)
    } else {
        xmlschema::parse_schema(&schema_src)
    };

    // A schema test asks whether the schema itself is valid.
    let Some(instance_path) = case.instance.as_ref() else {
        let answered_valid = parsed.is_ok();
        let matched = answered_valid == expected_valid;
        let detail = parsed.as_ref().err().map(|e| e.message.clone());
        // Rejecting a schema is a definite answer about that schema
        // even when parts of it would not have been enforced, but
        // *accepting* one whose constructs were skipped is not.
        if !gaps.is_empty() && answered_valid {
            let mut d = Decided::new(Outcome::Unsupported, matched);
            d.reason = reason;
            return d;
        }
        let mut d = judged(matched, expected_valid, detail);
        d.reason = reason;
        return d;
    };

    // An instance test needs the schema to have parsed at all.
    let schema = match parsed {
        Ok(schema) => schema,
        Err(e) => {
            return Decided::new(Outcome::Blocked, false)
                .because(format!("the schema did not parse: {}", e.message));
        }
    };
    let Ok(instance_src) = std::fs::read_to_string(instance_path) else {
        return Decided::new(Outcome::Blocked, false)
            .because("instance unreadable");
    };
    let Ok(instance) = oxml::parse(&instance_src) else {
        // The suite includes instances that are not well-formed, and
        // expects them to be invalid.
        let matched = !expected_valid;
        let mut d = judged(
            matched,
            expected_valid,
            Some("the instance is not well-formed XML".to_owned()),
        );
        d.reason = reason;
        return d;
    };

    let report = xmlschema::validate(&instance, &schema);
    let matched = report.is_valid() == expected_valid;
    let detail = report.violations.first().map(|v| v.message.clone());

    if gaps.is_empty() {
        let mut d = judged(matched, expected_valid, detail);
        d.reason = reason;
        return d;
    }
    // Something was skipped, so this answer is not evidence either
    // way -- including when it agrees.
    let mut d = Decided::new(Outcome::Unsupported, matched);
    d.reason = reason;
    d
}
