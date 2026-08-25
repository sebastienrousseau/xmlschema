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
    };

    // A panic is a result, not an abort: the point of running forty
    // thousand hostile documents is to find one.
    let outcome = catch_unwind(AssertUnwindSafe(|| decide(case)));
    match outcome {
        Ok((outcome, matched, reason)) => {
            record.outcome = outcome;
            record.would_have_matched = matched;
            record.reason = reason;
        }
        Err(_) => record.outcome = Outcome::Panic,
    }
    record
}

/// The outcome, whether an unsupported answer would have agreed, and
/// why the schema was not enforced in full.
fn decide(case: &Case) -> (Outcome, bool, Option<String>) {
    let Some(schema_path) = case.schema.as_ref() else {
        return (
            Outcome::Blocked,
            false,
            Some("the group names no schema document".into()),
        );
    };
    let Ok(schema_src) = std::fs::read_to_string(schema_path) else {
        return (Outcome::Blocked, false, Some("schema unreadable".into()));
    };

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

    let parsed = xmlschema::parse_schema(&schema_src);

    // A schema test asks whether the schema itself is valid.
    let Some(instance_path) = case.instance.as_ref() else {
        let answered_valid = parsed.is_ok();
        let matched = answered_valid == (case.expected == Expected::Valid);
        // Rejecting a schema is a definite answer about that schema
        // even when parts of it would not have been enforced, but
        // *accepting* one whose constructs were skipped is not.
        if !gaps.is_empty() && answered_valid {
            return (Outcome::Unsupported, matched, reason);
        }
        return (
            if matched {
                Outcome::Pass
            } else {
                Outcome::Fail
            },
            matched,
            reason,
        );
    };

    // An instance test needs the schema to have parsed at all.
    let Ok(schema) = parsed else {
        return (
            Outcome::Blocked,
            false,
            Some("the schema did not parse".into()),
        );
    };
    let Ok(instance_src) = std::fs::read_to_string(instance_path) else {
        return (Outcome::Blocked, false, Some("instance unreadable".into()));
    };
    let Ok(instance) = oxml::parse(&instance_src) else {
        // The suite includes instances that are not well-formed, and
        // expects them to be invalid.
        let matched = case.expected == Expected::Invalid;
        return (
            if matched {
                Outcome::Pass
            } else {
                Outcome::Fail
            },
            matched,
            reason,
        );
    };

    let report = xmlschema::validate(&instance, &schema);
    let matched = report.is_valid() == (case.expected == Expected::Valid);

    if gaps.is_empty() {
        return (
            if matched {
                Outcome::Pass
            } else {
                Outcome::Fail
            },
            matched,
            reason,
        );
    }
    // Something was skipped, so this answer is not evidence either
    // way -- including when it agrees.
    (Outcome::Unsupported, matched, reason)
}
