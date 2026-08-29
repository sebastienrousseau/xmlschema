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
    let postcode =
        Pattern::compile("[A-Z]{1,2}[0-9]{1,2}[A-Z]? ?[0-9][A-Z]{2}")
            .expect("a well-formed pattern");

    // The pattern keeps its source, which is what a diagnostic quotes.
    println!("pattern: {}", postcode.source());
    assert_eq!(
        postcode.source(),
        "[A-Z]{1,2}[0-9]{1,2}[A-Z]? ?[0-9][A-Z]{2}"
    );

    for value in ["SW1A 1AA", "EC1A1BB"] {
        println!("{value}: {}", postcode.matches(value));
        assert!(postcode.matches(value));
    }

    // XSD patterns are anchored at both ends, so a value that merely
    // *contains* a match is not a match.
    println!("`x SW1A 1AA`: {}", postcode.matches("x SW1A 1AA"));
    assert!(!postcode.matches("x SW1A 1AA"));
    assert!(
        !postcode.matches("sw1a 1aa"),
        "the character classes are upper case"
    );

    // Malformed syntax is reported rather than silently accepting
    // everything, which would make the facet enforce nothing.
    let bad = Pattern::compile("[A-Z");
    println!("`[A-Z`: {bad:?}");
    assert!(bad.is_err());
}
