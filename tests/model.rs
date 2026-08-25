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
fn every_built_in_is_its_own_type_with_its_own_rule() {
    // This test used to assert the opposite: that `byte`, `int`,
    // `long` and `short` all resolved to one unbounded `Integer`, and
    // that `NCName`, `ID` and `language` all resolved to `String`. Its
    // own comment warned that treating a type loosely "would let
    // invalid documents through silently" -- which is exactly what
    // the mapping it pinned did. `xs:byte` accepted 999.
    //
    // Identity is not the property worth pinning; behaviour is.
    let cases: &[(&str, &str, bool)] = &[
        // (type, value, accepted)
        ("byte", "127", true),
        ("byte", "128", false),
        ("byte", "-128", true),
        ("byte", "-129", false),
        ("short", "32767", true),
        ("short", "32768", false),
        ("int", "2147483647", true),
        ("int", "2147483648", false),
        ("long", "9223372036854775807", true),
        ("long", "9223372036854775808", false),
        ("unsignedByte", "255", true),
        ("unsignedByte", "256", false),
        ("unsignedByte", "-1", false),
        ("unsignedShort", "65535", true),
        ("unsignedShort", "65536", false),
        ("unsignedInt", "4294967295", true),
        ("unsignedInt", "4294967296", false),
        ("positiveInteger", "1", true),
        ("positiveInteger", "0", false),
        ("nonNegativeInteger", "0", true),
        ("nonNegativeInteger", "-1", false),
        ("negativeInteger", "-1", true),
        ("negativeInteger", "0", false),
        ("nonPositiveInteger", "0", true),
        ("nonPositiveInteger", "1", false),
        // Unbounded, so a value no machine integer holds is still valid.
        ("integer", "123456789012345678901234567890", true),
        ("integer", "1.0", false),
        ("decimal", "1.0", true),
        ("decimal", "1.0e3", false),
        ("double", "1.0e3", true),
        ("double", "NaN", true),
        ("double", "INF", true),
        // Name productions, which used to be plain strings.
        ("NCName", "abc", true),
        ("NCName", "a:b", false),
        ("NCName", "1abc", false),
        ("Name", "a:b", true),
        ("Name", "-abc", false),
        ("NMTOKEN", "-abc", true),
        ("NMTOKEN", "a b", false),
        ("NMTOKENS", "a b", true),
        ("language", "en-GB", true),
        ("language", "e0", false),
        ("QName", "p:local", true),
        ("QName", "p:l:x", false),
        // Whitespace is collapsed before validating, for every type
        // but string and normalizedString.
        ("integer", "  5  ", true),
        ("boolean", "\ttrue\n", true),
        // Dates and times, none of which existed before.
        ("date", "2001-02-28", true),
        ("date", "2001-02-29", false),
        ("date", "2004-02-29", true),
        ("date", "2001-13-01", false),
        ("time", "23:59:59", true),
        ("time", "24:00:00", true),
        ("time", "24:00:01", false),
        ("dateTime", "2001-01-01T00:00:00Z", true),
        ("dateTime", "2001-01-01", false),
        ("duration", "P1Y2M3DT4H5M6S", true),
        ("duration", "P", false),
        ("duration", "P1S", false),
        ("gYear", "2001", true),
        ("gMonth", "--02", true),
        ("gDay", "---15", true),
        ("gMonthDay", "--02-29", true),
        ("gYearMonth", "2001-02", true),
        ("hexBinary", "0FB7", true),
        ("hexBinary", "0FB", false),
        ("base64Binary", "QUJD", true),
        ("base64Binary", "QUJ", false),
    ];
    for (name, value, want) in cases {
        let ty = BuiltIn::from_name(name)
            .unwrap_or_else(|| panic!("`{name}` must resolve"));
        assert_eq!(
            ty.accepts(value),
            *want,
            "{name} should {} {value:?}",
            if *want { "accept" } else { "reject" }
        );
    }
}

/// Distinct types must not collapse into one another.
#[test]
fn the_integer_lattice_is_not_one_type() {
    let names = [
        "integer",
        "long",
        "int",
        "short",
        "byte",
        "nonNegativeInteger",
        "unsignedLong",
        "unsignedInt",
        "unsignedShort",
        "unsignedByte",
        "positiveInteger",
        "negativeInteger",
        "nonPositiveInteger",
    ];
    let mut seen = Vec::new();
    for n in names {
        let ty = BuiltIn::from_name(n).expect("resolves");
        assert!(!seen.contains(&ty), "`{n}` collapsed into another type");
        seen.push(ty);
    }
    assert_eq!(seen.len(), names.len());
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
