// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 xmlschema. All rights reserved.

//! What a built-in datatype knows about its own values.
//!
//! Run with:
//!
//! ```text
//! cargo run --example datatypes
//! ```

use std::cmp::Ordering;
use xmlschema::datatype::Datatype;

fn main() {
    let integer = Datatype::from_name("integer").expect("a built-in type");
    let date = Datatype::from_name("date").expect("a built-in type");
    let string = Datatype::from_name("string").expect("a built-in type");
    let nmtokens = Datatype::from_name("NMTOKENS").expect("a built-in type");

    // A list type answers what it is a list *of*; everything else
    // answers itself, so a caller need not check first.
    println!("NMTOKENS is a list of {:?}", nmtokens.item_type());
    assert_eq!(
        nmtokens.item_type(),
        Datatype::from_name("NMTOKEN").unwrap()
    );
    assert_eq!(integer.item_type(), integer);

    // Facets only mean something for types that have an order.
    assert!(integer.is_numeric() && integer.is_ordered());
    assert!(date.is_temporal() && date.is_ordered());
    assert!(
        !date.is_numeric(),
        "a date is ordered without being numeric"
    );
    assert!(!string.is_ordered(), "strings carry no order for facets");

    // Numbers compare as decimals, not through `f64`. Past 2^53 these
    // two values are distinct `xs:integer`s that floats call equal.
    let big_a = "999999999999999998";
    let big_b = "999999999999999999";
    println!("{big_a} vs {big_b}: {:?}", integer.compare(big_a, big_b));
    assert_eq!(integer.compare(big_a, big_b), Some(Ordering::Less));
    assert_eq!(
        big_a
            .parse::<f64>()
            .unwrap()
            .partial_cmp(&big_b.parse::<f64>().unwrap()),
        Some(Ordering::Equal),
        "which is exactly what comparing through f64 would have said"
    );

    // A value outside the type has no ordering rather than a wrong one.
    assert_eq!(integer.compare("x", "1"), None);

    // Temporal values share a scale, so they order against each other.
    println!(
        "1965-06-01 vs 1961-01-01: {:?}",
        date.compare("1965-06-01", "1961-01-01")
    );
    assert_eq!(
        date.compare("1965-06-01", "1961-01-01"),
        Some(Ordering::Greater)
    );

    let key = date.order_key("1970-01-01").expect("a date in the type");
    println!("order key for 1970-01-01: {key}");
    assert!(date.order_key("not a date").is_none());
    assert!(
        string.order_key("anything").is_none(),
        "unordered types have no key"
    );
}
