// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 xmlschema. All rights reserved.

//! Print the conformance figures, with their denominator.

use std::collections::BTreeMap;

use xmlschema_conformance::{
    REAL_TESTS, SUITE_RELEASE, catalog, data_dir, outcome::Outcome, runner,
};

fn main() -> Result<(), String> {
    let Some(root) = data_dir() else {
        return Err("run the `download` bin first".into());
    };
    let cases = catalog::load(&root)?;
    println!("suite      {SUITE_RELEASE}");
    println!("tests      {} (expected {REAL_TESTS})", cases.len());

    let (records, counts) = runner::run_all(&cases);
    println!("\noverall    {counts}");
    println!(
        "\nhad unsupported answers been counted as passes whenever they\n\
         happened to agree, this would read {:.1}% — {} tests where\n\
         nothing was enforced.",
        counts.flattered_rate(),
        counts.vacuous
    );

    let mut by_set: BTreeMap<&str, (usize, usize, usize)> = BTreeMap::new();
    for r in &records {
        let e = by_set.entry(r.set.as_str()).or_default();
        match r.outcome {
            Outcome::Pass => {
                e.0 += 1;
                e.1 += 1;
            }
            Outcome::Fail | Outcome::Panic => e.1 += 1,
            Outcome::Unsupported | Outcome::Blocked => e.2 += 1,
        }
    }
    println!("\nby test set:");
    for (set, (pass, decided, skipped)) in &by_set {
        let rate = if *decided == 0 {
            0.0
        } else {
            *pass as f64 * 100.0 / *decided as f64
        };
        println!(
            "  {set:<28} {rate:>5.1}% of {decided:>5} decided \
             ({skipped} not enforced)"
        );
    }

    // Failures, split by direction. Wrongly rejecting a valid
    // document breaks a caller whose schema is correct; wrongly
    // accepting an invalid one is a check that is missing. They are
    // not the same severity and are not fixed the same way.
    let rejected: Vec<&runner::Record> = records
        .iter()
        .filter(|r| r.direction == Some(runner::Direction::WronglyRejected))
        .collect();
    let accepted: Vec<&runner::Record> = records
        .iter()
        .filter(|r| r.direction == Some(runner::Direction::WronglyAccepted))
        .collect();
    println!(
        "\nfailures: {} wrongly rejected (valid, called invalid), \
         {} wrongly accepted (invalid, called valid)",
        rejected.len(),
        accepted.len()
    );

    // Wrongly *rejected* carries a violation message, which says what
    // went wrong. Wrongly *accepted* carries none -- there were no
    // violations, which is the problem -- so those group by what the
    // suite's own test name says the case is about.
    let mut causes: BTreeMap<String, usize> = BTreeMap::new();
    for r in &rejected {
        let detail = r.detail.as_deref().unwrap_or("(no message)");
        let key: String = detail
            .split(|c: char| c == '`' || c == '\'')
            .step_by(2)
            .collect::<Vec<_>>()
            .join("_")
            .chars()
            .take(72)
            .collect();
        *causes.entry(key).or_default() += 1;
    }
    let mut ranked: Vec<_> = causes.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1));
    println!("\nwrongly rejected, by message:");
    for (cause, n) in ranked.iter().take(15) {
        println!("  {n:>6}  {cause}");
    }

    // The shape of a message says what kind of thing went wrong; an
    // example says what actually did. Both are needed to fix one.
    for (top, _) in ranked.iter().take(3) {
        println!("\nexamples of `{top}`:");
        let mut shown = 0;
        for r in &rejected {
            let Some(detail) = r.detail.as_deref() else {
                continue;
            };
            let key: String = detail
                .split(|c: char| c == '`' || c == '\'')
                .step_by(2)
                .collect::<Vec<_>>()
                .join("_")
                .chars()
                .take(72)
                .collect();
            if key != *top {
                continue;
            }
            println!("  {}  {detail}", r.id);
            shown += 1;
            if shown == 4 {
                break;
            }
        }
    }

    let mut families: BTreeMap<String, usize> = BTreeMap::new();
    for r in &accepted {
        // The suite names a group by feature and number, so the
        // leading letters identify what is being tested.
        let head = r.id.split('/').next().unwrap_or(&r.id);
        let family: String =
            head.chars().take_while(|c| !c.is_ascii_digit()).collect();
        let family = if family.is_empty() {
            head.chars().take(24).collect()
        } else {
            family
        };
        *families
            .entry(format!("{family}  [{}]", r.set))
            .or_default() += 1;
    }
    let mut ranked: Vec<_> = families.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1));
    println!("\nwrongly accepted, by test family:");
    for (family, n) in ranked.iter().take(20) {
        println!("  {n:>6}  {family}");
    }

    let mut reasons: BTreeMap<&str, usize> = BTreeMap::new();
    for r in &records {
        if r.outcome == Outcome::Unsupported {
            if let Some(reason) = r.reason.as_deref() {
                let head = reason.split_once(": ").map_or(reason, |(c, _)| c);
                *reasons.entry(head).or_default() += 1;
            }
        }
    }
    let mut ranked: Vec<_> = reasons.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1));
    println!("\nwhat stops enforcement, most common first:");
    for (reason, n) in ranked.iter().take(25) {
        println!("  {n:>6}  {reason}");
    }
    Ok(())
}
