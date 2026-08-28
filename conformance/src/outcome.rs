// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 xmlschema. All rights reserved.

//! What happened to one test, and how the totals are reported.

use std::fmt;

/// The result of running one test.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Outcome {
    /// The schema is enforced in full and the answer matched.
    Pass,
    /// The schema is enforced in full and the answer did not match.
    Fail,
    /// The schema uses a construct this crate does not enforce, so the
    /// answer means nothing either way.
    Unsupported,
    /// The harness could not run the test — a missing file, or a
    /// schema this crate rejects outright.
    Blocked,
    /// The crate panicked.
    Panic,
}

/// Totals across a run.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// Enforced in full, answer matched.
    pub pass: usize,
    /// Enforced in full, answer did not match.
    pub fail: usize,
    /// A construct was skipped, so the answer was not evidence.
    pub unsupported: usize,
    /// Could not be run.
    pub blocked: usize,
    /// Panicked.
    pub panic: usize,
    /// Unsupported tests whose answer *would* have matched.
    ///
    /// Reported because it is the size of the flattery avoided: these
    /// are the tests a harness that ignored coverage would have
    /// counted as passes without any constraint being enforced.
    pub vacuous: usize,
}

impl core::fmt::Display for Outcome {
    /// The word written into a baseline row.
    ///
    /// Stable by contract: a baseline is a committed file, so renaming
    /// an outcome here would read as every test having changed at
    /// once.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::Unsupported => "unsupported",
            Self::Blocked => "blocked",
            Self::Panic => "panic",
        })
    }
}

impl Counts {
    /// Record one outcome.
    ///
    /// `would_have_matched` only means anything for
    /// [`Outcome::Unsupported`], where it says whether the unchecked
    /// answer happened to agree with the expectation.
    pub fn add(&mut self, outcome: Outcome, would_have_matched: bool) {
        match outcome {
            Outcome::Pass => self.pass += 1,
            Outcome::Fail => self.fail += 1,
            Outcome::Unsupported => {
                self.unsupported += 1;
                if would_have_matched {
                    self.vacuous += 1;
                }
            }
            Outcome::Blocked => self.blocked += 1,
            Outcome::Panic => self.panic += 1,
        }
    }

    /// Every test seen.
    #[must_use]
    pub const fn total(&self) -> usize {
        self.pass + self.fail + self.unsupported + self.blocked + self.panic
    }

    /// Tests where the answer was evidence of something.
    #[must_use]
    pub const fn decided(&self) -> usize {
        self.pass + self.fail + self.panic
    }

    /// Pass rate over decided tests.
    ///
    /// Meaningless without [`Counts::coverage`], which is why
    /// [`fmt::Display`] never prints one without the other.
    #[must_use]
    pub fn pass_rate(&self) -> f64 {
        if self.decided() == 0 {
            return 0.0;
        }
        // A count that exceeded f64's 53-bit mantissa would need
        // 9,007,199,254,740,993 tests. The suite has 39,420.
        #[allow(clippy::cast_precision_loss)]
        let rate = self.pass as f64 * 100.0 / self.decided() as f64;
        rate
    }

    /// Share of the suite that reached a decision.
    ///
    /// The denominator made visible. A subset implementation scores a
    /// high pass rate trivially by deciding little, so this is the
    /// number that says how much the pass rate covers.
    #[must_use]
    pub fn coverage(&self) -> f64 {
        if self.total() == 0 {
            return 0.0;
        }
        #[allow(clippy::cast_precision_loss)]
        let rate = self.decided() as f64 * 100.0 / self.total() as f64;
        rate
    }

    /// What the pass rate would have been had unsupported tests been
    /// counted as passes whenever the answer happened to agree.
    ///
    /// Published alongside the real figure so the gap between them is
    /// visible rather than a claim.
    #[must_use]
    pub fn flattered_rate(&self) -> f64 {
        let decided = self.decided() + self.unsupported;
        if decided == 0 {
            return 0.0;
        }
        #[allow(clippy::cast_precision_loss)]
        let rate = (self.pass + self.vacuous) as f64 * 100.0 / decided as f64;
        rate
    }
}

impl fmt::Display for Counts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} pass, {} fail, {} panic, {} unsupported, {} blocked \
             — {:.1}% of {} decided ({:.1}% coverage of {})",
            self.pass,
            self.fail,
            self.panic,
            self.unsupported,
            self.blocked,
            self.pass_rate(),
            self.decided(),
            self.coverage(),
            self.total(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unsupported_test_is_never_a_pass() {
        let mut c = Counts::default();
        // The answer agreed, but nothing was enforced.
        c.add(Outcome::Unsupported, true);
        assert_eq!(c.pass, 0);
        assert_eq!(c.unsupported, 1);
        assert_eq!(c.vacuous, 1);
        assert_eq!(c.decided(), 0);
        assert!(
            c.pass_rate().abs() < f64::EPSILON,
            "an unchecked answer is not a pass"
        );
    }

    #[test]
    fn the_flattered_rate_shows_what_was_avoided() {
        let mut c = Counts::default();
        c.add(Outcome::Pass, false);
        for _ in 0..9 {
            c.add(Outcome::Unsupported, true);
        }
        // Honest: one decided test, one pass.
        assert_eq!(c.decided(), 1);
        assert!((c.pass_rate() - 100.0).abs() < f64::EPSILON);
        assert!(
            (c.coverage() - 10.0).abs() < f64::EPSILON,
            "one of ten tests decided anything"
        );
        // Flattered: ten "passes" out of ten.
        assert!((c.flattered_rate() - 100.0).abs() < f64::EPSILON);
        assert_eq!(c.vacuous, 9, "nine answers nothing was checked for");
    }

    #[test]
    fn a_panic_counts_against_the_pass_rate_rather_than_being_set_aside() {
        let mut c = Counts::default();
        c.add(Outcome::Pass, false);
        c.add(Outcome::Panic, false);
        assert_eq!(c.decided(), 2);
        assert!((c.pass_rate() - 50.0).abs() < f64::EPSILON);
    }
}
