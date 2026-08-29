// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 xmlschema. All rights reserved.

//! Compiling and applying an `xs:pattern` facet.
//!
//! Run with:
//!
//! ```text
//! cargo run --example patterns
//! ```

use xmlschema::pattern::Pattern;

fn main() {
    // A part number: two letters, a dash, four digits, and an
    // optional revision letter.
    let part_number = Pattern::compile("[A-Z]{2}-[0-9]{4}[A-Z]?")
        .expect("a well-formed pattern");

    // The pattern keeps its source, which is what a diagnostic quotes.
    println!("pattern: {}", part_number.source());
    assert_eq!(part_number.source(), "[A-Z]{2}-[0-9]{4}[A-Z]?");

    for value in ["XY-0042", "AB-1234C"] {
        println!("{value}: {}", part_number.matches(value));
        assert!(part_number.matches(value));
    }

    // XSD patterns are anchored at both ends, so a value that merely
    // *contains* a match is not a match.
    println!("`part XY-0042`: {}", part_number.matches("part XY-0042"));
    assert!(!part_number.matches("part XY-0042"));
    assert!(
        !part_number.matches("xy-0042"),
        "the character classes are upper case"
    );
    assert!(
        !part_number.matches("XY-042"),
        "the digit count is exact, not a minimum"
    );

    // Malformed syntax is reported rather than silently accepting
    // everything, which would make the facet enforce nothing.
    let bad = Pattern::compile("[A-Z");
    println!("`[A-Z`: {bad:?}");
    assert!(bad.is_err());
}
