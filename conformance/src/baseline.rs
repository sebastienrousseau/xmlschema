// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (c) 2026 xmlschema. All rights reserved.

//! The baseline ratchet.
//!
//! Results are compared against a committed file rather than asserted
//! against a target. A regression fails, and so does an
//! **improvement** — which sounds perverse until you have watched a
//! pass rate drift upward because a test started being skipped rather
//! than passing. Requiring the baseline to be regenerated deliberately
//! makes every change to the number a reviewed change.
//!
//! Ported from `oxml`, which has run this arrangement for several
//! releases. Before it existed here, this crate published "95.6% of
//! 39,420 tests" with an empty `baselines/` directory, no test that
//! ran the suite, and no CI job — a figure produced by invoking a
//! binary by hand and typing the result into a document.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::outcome::{Counts, Outcome};
use crate::runner::Record;

/// Render results as the baseline format.
///
/// One line per non-passing test, sorted, with a header carrying the
/// counts. Passing tests are omitted: there are more than 33,000 of
/// them and listing them would bury the signal.
///
/// `detail` is written for a human and is **not** compared, because
/// error wording changes far more often than behaviour does.
#[must_use]
pub fn render(results: &[Record], counts: &Counts) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "#counts\tpass={}\tfail={}\tpanic={}\tunsupported={}\tblocked={}\tvacuous={}\ttotal={}",
        counts.pass,
        counts.fail,
        counts.panic,
        counts.unsupported,
        counts.blocked,
        counts.vacuous,
        counts.total(),
    );
    let mut rows: Vec<&Record> = results
        .iter()
        .filter(|r| r.outcome != Outcome::Pass)
        .collect();
    rows.sort_by(|a, b| a.id.cmp(&b.id));
    for r in rows {
        let _ = writeln!(
            out,
            "{}\t{}\t{}\t{}",
            r.set,
            r.id,
            r.outcome,
            r.detail.as_deref().unwrap_or("")
        );
    }
    out
}

/// The comparable part of a baseline: counts, and outcome by test id.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Parsed {
    /// The `#counts` header.
    pub counts: BTreeMap<String, usize>,
    /// Outcome per test id, excluding passes.
    pub outcomes: BTreeMap<String, String>,
}

/// Read a baseline file.
#[must_use]
pub fn parse(text: &str) -> Parsed {
    let mut p = Parsed::default();
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("#counts\t") {
            for field in rest.split('\t') {
                if let Some((k, Ok(n))) =
                    field.split_once('=').map(|(k, v)| (k, v.parse()))
                {
                    let _ = p.counts.insert(k.to_owned(), n);
                }
            }
            continue;
        }
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let mut f = line.split('\t');
        let (Some(_set), Some(id), Some(outcome)) =
            (f.next(), f.next(), f.next())
        else {
            continue;
        };
        let _ = p.outcomes.insert(id.to_owned(), outcome.to_owned());
    }
    p
}

/// Compare a fresh run against a baseline, returning every difference.
#[must_use]
pub fn diff(baseline: &Parsed, current: &Parsed) -> Vec<String> {
    let mut out = Vec::new();

    for (k, want) in &baseline.counts {
        let got = current.counts.get(k).copied().unwrap_or(0);
        if got != *want {
            out.push(format!("count `{k}`: baseline {want}, now {got}"));
        }
    }

    for (id, want) in &baseline.outcomes {
        match current.outcomes.get(id) {
            None => out.push(format!("{id}: was {want}, now passes")),
            Some(got) if got != want => {
                out.push(format!("{id}: was {want}, now {got}"));
            }
            Some(_) => {}
        }
    }
    for (id, got) in &current.outcomes {
        if !baseline.outcomes.contains_key(id) {
            out.push(format!("{id}: was passing, now {got}"));
        }
    }
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(text: &str) -> Parsed {
        parse(text)
    }

    #[test]
    fn a_regression_is_reported() {
        let before = parsed("#counts\tpass=2\tfail=0\ttotal=2\n");
        let after =
            parsed("#counts\tpass=1\tfail=1\ttotal=2\nsetA\tt1\tfail\twhy\n");
        let d = diff(&before, &after);
        assert!(d.iter().any(|s| s.contains("count `pass`")), "{d:?}");
        assert!(d.iter().any(|s| s.contains("t1")), "{d:?}");
    }

    /// An improvement fails too. That is the point of a ratchet.
    #[test]
    fn an_unreviewed_improvement_is_reported() {
        let before =
            parsed("#counts\tpass=1\tfail=1\ttotal=2\nsetA\tt1\tfail\twhy\n");
        let after = parsed("#counts\tpass=2\tfail=0\ttotal=2\n");
        let d = diff(&before, &after);
        assert!(
            d.iter().any(|s| s.contains("now passes")),
            "an improvement must be reviewed, not absorbed: {d:?}"
        );
    }

    #[test]
    fn an_unchanged_run_is_silent() {
        let text = "#counts\tpass=1\tfail=1\ttotal=2\nsetA\tt1\tfail\twhy\n";
        assert!(diff(&parsed(text), &parsed(text)).is_empty());
    }

    /// The detail column is for a human and must not be compared.
    #[test]
    fn wording_changes_do_not_fail_the_ratchet() {
        let before = parsed(
            "#counts\tpass=0\tfail=1\ttotal=1\nsetA\tt1\tfail\told wording\n",
        );
        let after = parsed(
            "#counts\tpass=0\tfail=1\ttotal=1\nsetA\tt1\tfail\tnew wording\n",
        );
        assert!(diff(&before, &after).is_empty());
    }
}
