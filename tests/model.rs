// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 xmlschema. All rights reserved.

//! The schema model: cardinality, built-in type resolution, and the
//! `Display` output that every diagnostic is built from.

use xmlschema::{BuiltIn, Facets, Occurs};

#[test]
fn the_default_cardinality_is_exactly_once() {
    // XSD's default when neither minOccurs nor maxOccurs is written.
    // Getting this wrong silently makes every element optional.
    let d = Occurs::default();
    assert_eq!(d.min, 1);
    assert_eq!(d.max, Some(1));
    assert!(d.permits(1));
    assert!(!d.permits(0));
    assert!(!d.permits(2));
}

#[test]
fn permits_respects_both_bounds() {
    let between = Occurs {
        min: 2,
        max: Some(4),
    };
    assert!(!between.permits(1));
    assert!(between.permits(2));
    assert!(between.permits(4));
    assert!(!between.permits(5));

    let unbounded = Occurs { min: 1, max: None };
    assert!(!unbounded.permits(0));
    assert!(unbounded.permits(1));
    assert!(unbounded.permits(10_000));
}

#[test]
fn describe_covers_every_shape_of_cardinality() {
    // These strings end up in user-facing violation messages, so each
    // combination needs to read correctly rather than merely exist.
    let cases = [
        (
            Occurs {
                min: 1,
                max: Some(1),
            },
            "exactly once",
        ),
        (
            Occurs {
                min: 0,
                max: Some(1),
            },
            "at most once",
        ),
        (Occurs { min: 0, max: None }, "any number of times"),
        (Occurs { min: 2, max: None }, "at least 2 times"),
        (
            Occurs {
                min: 3,
                max: Some(3),
            },
            "exactly 3 times",
        ),
        (
            Occurs {
                min: 1,
                max: Some(4),
            },
            "between 1 and 4 times",
        ),
    ];
    for (occurs, expected) in cases {
        assert_eq!(occurs.describe(), expected, "for {occurs:?}");
    }
}

#[test]
fn built_in_names_resolve_ignoring_the_prefix() {
    // Schemas bind the XSD namespace to any prefix they like, and some
    // bind it as the default with no prefix at all.
    assert_eq!(BuiltIn::from_name("xs:string"), Some(BuiltIn::String));
    assert_eq!(BuiltIn::from_name("xsd:string"), Some(BuiltIn::String));
    assert_eq!(BuiltIn::from_name("string"), Some(BuiltIn::String));
}

#[test]
fn every_built_in_family_maps_to_its_validation_rule() {
    // The aliases matter: `int` and `long` validate as integers, and
    // treating an unrecognised one as "no constraint" would let invalid
    // documents through silently.
    let string_like = [
        "string",
        "normalizedString",
        "token",
        "NMTOKEN",
        "Name",
        "NCName",
        "ID",
        "IDREF",
        "language",
    ];
    for n in string_like {
        assert_eq!(BuiltIn::from_name(n), Some(BuiltIn::String), "{n}");
    }

    for n in ["integer", "int", "long", "short", "byte"] {
        assert_eq!(BuiltIn::from_name(n), Some(BuiltIn::Integer), "{n}");
    }

    for n in [
        "nonNegativeInteger",
        "positiveInteger",
        "unsignedInt",
        "unsignedLong",
        "unsignedShort",
    ] {
        assert_eq!(
            BuiltIn::from_name(n),
            Some(BuiltIn::NonNegativeInteger),
            "{n}"
        );
    }

    for n in ["double", "float"] {
        assert_eq!(BuiltIn::from_name(n), Some(BuiltIn::Double), "{n}");
    }

    assert_eq!(BuiltIn::from_name("boolean"), Some(BuiltIn::Boolean));
    assert_eq!(BuiltIn::from_name("decimal"), Some(BuiltIn::Decimal));
    assert_eq!(BuiltIn::from_name("date"), Some(BuiltIn::Date));
    assert_eq!(BuiltIn::from_name("dateTime"), Some(BuiltIn::DateTime));
    assert_eq!(BuiltIn::from_name("anyURI"), Some(BuiltIn::AnyUri));
}

#[test]
fn an_unknown_type_name_is_none_rather_than_a_guess() {
    for n in ["", "notAType", "xs:", "xs:madeUp", "String", "INTEGER"] {
        assert_eq!(BuiltIn::from_name(n), None, "{n} should not resolve");
    }
}

#[test]
fn facets_are_empty_until_one_is_set() {
    assert!(Facets::default().is_empty());

    let with_enum = Facets {
        enumeration: vec!["a".to_owned()],
        ..Default::default()
    };
    assert!(!with_enum.is_empty());

    let with_min = Facets {
        min_length: Some(1),
        ..Default::default()
    };
    assert!(!with_min.is_empty());
}
