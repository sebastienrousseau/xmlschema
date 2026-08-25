// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 xmlschema. All rights reserved.

//! The pattern engine.
//!
//! Matching is backtracking, so the cost is in the shapes that make it
//! backtrack: nested quantifiers, and alternations whose branches
//! share a prefix. A pattern is compiled once per validation and
//! applied to every value, so both halves are measured.

#![allow(missing_docs)] // criterion_group! generates an undocumented item

use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use xmlschema::Pattern;

fn bench(c: &mut Criterion) {
    let cases: &[(&str, &str, &str)] = &[
        // (name, pattern, value)
        ("literal", "[a-z]{3}-[0-9]{4}", "abc-1234"),
        ("alternation", "(cat|category|catalogue)s?", "catalogues"),
        ("nested_quantifier", "(a+b?)+c", "aaabaaabaaabc"),
        ("ncname", r"[\i-[:]][\c-[:]]*", "a-long_name.with.parts"),
        ("unicode_class", r"\p{L}+\d*", "Ünïcödé123"),
    ];

    let mut group = c.benchmark_group("pattern");
    for (name, pattern, value) in cases {
        let _ = group.bench_function(format!("compile_{name}"), |b| {
            b.iter(|| Pattern::compile(black_box(pattern)).unwrap());
        });
        let compiled = Pattern::compile(pattern).expect("compiles");
        let _ = group.bench_function(format!("match_{name}"), |b| {
            b.iter(|| compiled.matches(black_box(value)));
        });
    }
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
