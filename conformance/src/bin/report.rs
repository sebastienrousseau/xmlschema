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
