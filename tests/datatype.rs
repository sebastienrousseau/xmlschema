// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 xmlschema. All rights reserved.

//! The lexical and value rules of the XSD built-in datatypes.
//!
//! Each type is checked on the values that distinguish it from its
//! nearest relative, because that is where a collapsed lattice used to
//! agree by accident: `xs:byte` differs from `xs:integer` only past
//! 127, and `xs:NCName` from `xs:string` only when a colon appears.

use xmlschema::datatype::{Datatype, WhiteSpace};

fn ty(name: &str) -> Datatype {
    Datatype::from_name(name)
        .unwrap_or_else(|| panic!("`{name}` should resolve"))
}

#[test]
fn a_prefix_is_ignored_when_resolving_a_name() {
    assert_eq!(ty("xs:integer"), ty("integer"));
    assert_eq!(ty("xsd:integer"), ty("integer"));
    assert_eq!(Datatype::from_name("notAType"), None);
    assert_eq!(Datatype::from_name(""), None);
}

#[test]
fn whitespace_processing_differs_by_type() {
    assert_eq!(ty("string").white_space(), WhiteSpace::Preserve);
    assert_eq!(ty("normalizedString").white_space(), WhiteSpace::Replace);
    assert_eq!(ty("token").white_space(), WhiteSpace::Collapse);
    assert_eq!(ty("integer").white_space(), WhiteSpace::Collapse);

    // Preserve keeps the value exactly; replace turns tabs into
    // spaces; collapse also squeezes runs and trims.
    assert_eq!(ty("string").normalise("  a\tb  "), "  a\tb  ");
    assert_eq!(ty("normalizedString").normalise("a\tb"), "a b");
    assert_eq!(ty("token").normalise("  a\t\tb  "), "a b");
}

#[test]
fn booleans_take_four_forms_and_no_others() {
    for v in ["true", "false", "1", "0"] {
        assert!(ty("boolean").accepts(v), "{v}");
    }
    for v in ["True", "FALSE", "yes", "2", ""] {
        assert!(!ty("boolean").accepts(v), "{v}");
    }
}

#[test]
fn numeric_lexical_forms_are_distinguished() {
    assert!(ty("integer").accepts("+5"));
    assert!(ty("integer").accepts("-5"));
    assert!(ty("integer").accepts("007"));
    assert!(!ty("integer").accepts("5.0"));
    assert!(!ty("integer").accepts("5e3"));
    assert!(!ty("integer").accepts("+"));
    assert!(!ty("integer").accepts(""));

    assert!(ty("decimal").accepts("5.0"));
    assert!(ty("decimal").accepts(".5"));
    assert!(ty("decimal").accepts("5."));
    assert!(!ty("decimal").accepts("5e3"));
    assert!(!ty("decimal").accepts("."));

    assert!(ty("double").accepts("5e3"));
    assert!(ty("double").accepts("-5.0E-3"));
    assert!(ty("double").accepts("INF"));
    assert!(ty("double").accepts("-INF"));
    assert!(ty("double").accepts("NaN"));
    assert!(!ty("double").accepts("inf"));
    assert!(!ty("double").accepts("5e"));
}

#[test]
fn hex_and_base64_check_their_shape() {
    assert!(ty("hexBinary").accepts("0FB7"));
    assert!(ty("hexBinary").accepts(""));
    assert!(!ty("hexBinary").accepts("0FB"), "odd length");
    assert!(!ty("hexBinary").accepts("0FBG"), "not a hex digit");

    assert!(ty("base64Binary").accepts("QUJD"));
    assert!(ty("base64Binary").accepts("QQ=="));
    assert!(!ty("base64Binary").accepts("QUJ"), "not a multiple of four");
    assert!(!ty("base64Binary").accepts("QU*D"));
}

#[test]
fn names_follow_the_xml_productions() {
    assert!(ty("Name").accepts("a:b"));
    assert!(ty("Name").accepts("_x"));
    assert!(
        !ty("Name").accepts("1a"),
        "a name may not start with a digit"
    );
    assert!(!ty("Name").accepts("-a"));
    assert!(!ty("Name").accepts(""));

    assert!(ty("NCName").accepts("abc"));
    assert!(!ty("NCName").accepts("a:b"), "no colon in an NCName");

    assert!(ty("NMTOKEN").accepts("-a.1"));
    assert!(!ty("NMTOKEN").accepts("a b"));
    assert!(!ty("NMTOKEN").accepts(""));

    assert!(ty("NMTOKENS").accepts("a b c"));
    assert!(!ty("NMTOKENS").accepts(""), "the empty list is not a value");

    assert!(ty("QName").accepts("p:l"));
    assert!(ty("QName").accepts("l"));
    assert!(!ty("QName").accepts("p:l:x"));
    assert!(!ty("QName").accepts(":l"));
}

#[test]
fn language_tags_follow_rfc_3066() {
    assert!(ty("language").accepts("en"));
    assert!(ty("language").accepts("en-GB"));
    assert!(ty("language").accepts("x-klingon-1"));
    assert!(
        !ty("language").accepts("e0"),
        "the primary tag is alphabetic"
    );
    assert!(!ty("language").accepts("en-"), "an empty subtag");
    assert!(!ty("language").accepts("toolongprimary"));
}

#[test]
fn the_calendar_is_real() {
    assert!(ty("date").accepts("2004-02-29"), "a leap year");
    assert!(!ty("date").accepts("2001-02-29"), "not a leap year");
    assert!(!ty("date").accepts("1900-02-29"), "a century that is not");
    assert!(ty("date").accepts("2000-02-29"), "a century that is");
    assert!(!ty("date").accepts("2001-04-31"), "April has thirty days");
    assert!(!ty("date").accepts("2001-13-01"));
    assert!(!ty("date").accepts("0000-01-01"), "there is no year zero");
    // Timezones are permitted on every date and time form.
    assert!(ty("date").accepts("2001-01-01Z"));
    assert!(ty("date").accepts("2001-01-01+05:30"));
    assert!(ty("date").accepts("-2001-01-01"), "a negative year");
}

#[test]
fn times_permit_midnight_at_the_end_of_a_day() {
    assert!(ty("time").accepts("00:00:00"));
    assert!(ty("time").accepts("23:59:59.999"));
    assert!(ty("time").accepts("24:00:00"), "the end of a day");
    assert!(!ty("time").accepts("24:00:01"), "and nothing past it");
    assert!(!ty("time").accepts("23:60:00"));
    assert!(!ty("time").accepts("23:59"), "seconds are not optional");
    assert!(!ty("time").accepts("23:59:59."), "an empty fraction");
}

#[test]
fn durations_need_a_component_and_an_order() {
    assert!(ty("duration").accepts("P1Y2M3DT4H5M6S"));
    assert!(ty("duration").accepts("-P1Y"));
    assert!(ty("duration").accepts("PT0.5S"));
    assert!(!ty("duration").accepts("P"), "no component");
    assert!(!ty("duration").accepts("P1YT"), "a T with no time");
    assert!(!ty("duration").accepts("P1S"), "S belongs after T");
    assert!(!ty("duration").accepts("1Y"), "no P");
    assert!(
        !ty("duration").accepts("P1.5Y"),
        "only seconds may be fractional"
    );
}

#[test]
fn the_gregorian_forms_are_distinct() {
    assert!(ty("gYear").accepts("2001"));
    assert!(ty("gMonth").accepts("--02"));
    assert!(ty("gMonth").accepts("--02--"), "the original 1.0 form");
    assert!(ty("gDay").accepts("---15"));
    assert!(ty("gMonthDay").accepts("--02-29"));
    assert!(ty("gYearMonth").accepts("2001-02"));
    assert!(!ty("gMonth").accepts("02"));
    assert!(!ty("gDay").accepts("--15"));
    assert!(!ty("gMonthDay").accepts("--13-01"));
}

#[test]
fn only_ordered_types_have_an_ordering() {
    assert!(ty("integer").is_ordered());
    assert!(ty("date").is_ordered());
    assert!(ty("duration").is_ordered());
    assert!(!ty("string").is_ordered());
    assert!(!ty("boolean").is_ordered());
    assert!(!ty("hexBinary").is_ordered());

    assert!(ty("date").is_temporal());
    assert!(!ty("integer").is_temporal());
    assert!(ty("integer").is_numeric());
    assert!(!ty("date").is_numeric());
}

#[test]
fn comparison_is_exact_and_type_aware() {
    use std::cmp::Ordering;
    let int = ty("integer");
    assert_eq!(int.compare("1", "2"), Some(Ordering::Less));
    assert_eq!(int.compare("2", "1"), Some(Ordering::Greater));
    assert_eq!(int.compare("+1", "1"), Some(Ordering::Equal));
    assert_eq!(int.compare("007", "7"), Some(Ordering::Equal));
    assert_eq!(int.compare("-0", "0"), Some(Ordering::Equal));
    assert_eq!(int.compare("-1", "1"), Some(Ordering::Less));
    // Past the point an f64 can distinguish.
    assert_eq!(
        int.compare("999999999999999998", "999999999999999999"),
        Some(Ordering::Less)
    );

    let dec = ty("decimal");
    assert_eq!(dec.compare("1.10", "1.1"), Some(Ordering::Equal));
    assert_eq!(dec.compare("1.5", "1.45"), Some(Ordering::Greater));

    let date = ty("date");
    assert_eq!(
        date.compare("2000-12-31", "2001-01-01"),
        Some(Ordering::Less)
    );
    assert_eq!(
        date.compare("2000-02-29", "2000-03-01"),
        Some(Ordering::Less)
    );

    // A value the type does not accept has no place in the ordering.
    assert_eq!(int.compare("x", "1"), None);
}

#[test]
fn the_built_in_list_types_know_their_item() {
    assert!(ty("NMTOKENS").is_built_in_list());
    assert!(ty("IDREFS").is_built_in_list());
    assert!(ty("ENTITIES").is_built_in_list());
    assert!(!ty("NMTOKEN").is_built_in_list());
    assert_eq!(ty("NMTOKENS").item_type(), ty("NMTOKEN"));
    assert_eq!(ty("IDREFS").item_type(), ty("IDREF"));
    // Everything else is its own item type, so a caller need not ask.
    assert_eq!(ty("integer").item_type(), ty("integer"));
}

/// Every built-in resolves, describes itself, and is distinct.
///
/// A diagnostic naming the wrong type is worse than none, and a
/// description is the only place several of these types appear.
#[test]
fn every_built_in_resolves_and_describes_itself() {
    const ALL: &[&str] = &[
        "anySimpleType",
        "anyType",
        "string",
        "normalizedString",
        "token",
        "language",
        "NMTOKEN",
        "NMTOKENS",
        "Name",
        "NCName",
        "ID",
        "IDREF",
        "IDREFS",
        "ENTITY",
        "ENTITIES",
        "boolean",
        "decimal",
        "integer",
        "nonPositiveInteger",
        "negativeInteger",
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
        "float",
        "double",
        "duration",
        "dateTime",
        "time",
        "date",
        "gYearMonth",
        "gYear",
        "gMonthDay",
        "gMonth",
        "gDay",
        "hexBinary",
        "base64Binary",
        "anyURI",
        "QName",
        "NOTATION",
    ];
    let mut described = Vec::new();
    for name in ALL {
        let t = ty(name);
        let text = t.describe();
        assert!(!text.is_empty(), "{name} describes itself as nothing");
        described.push((t, text));
    }
    // `anyType` is a spelling of `anySimpleType`; every other name is
    // its own type.
    let mut kinds: Vec<_> = described.iter().map(|(t, _)| *t).collect();
    kinds.sort_by_key(|t| format!("{t:?}"));
    kinds.dedup();
    assert_eq!(kinds.len(), ALL.len() - 1, "one alias, no other collapse");
}

/// The types with no constraint accept anything, including nothing.
#[test]
fn the_unconstrained_types_accept_anything() {
    for name in ["anySimpleType", "anyType", "string", "token", "anyURI"] {
        for value in ["", "anything at all", "  spaced  ", "<>&"] {
            assert!(ty(name).accepts(value), "{name} should accept {value:?}");
        }
    }
}

/// Every temporal type has an ordering, and it has to be the right one.
#[test]
fn temporal_ordering_covers_every_form() {
    use std::cmp::Ordering::{Greater, Less};
    let cases: &[(&str, &str, &str)] = &[
        // (type, smaller, larger)
        ("date", "2000-12-31", "2001-01-01"),
        ("date", "-0001-01-01", "0001-01-01"),
        ("dateTime", "2001-01-01T00:00:00", "2001-01-01T00:00:01"),
        ("dateTime", "2001-01-01T23:59:59", "2001-01-02T00:00:00"),
        ("time", "00:00:00", "23:59:59"),
        ("time", "12:00:00", "12:00:01"),
        ("gYear", "1999", "2000"),
        ("gYearMonth", "2001-01", "2001-02"),
        ("gMonth", "--01", "--12"),
        ("gDay", "---01", "---31"),
        ("gMonthDay", "--01-01", "--12-31"),
        ("duration", "P1D", "P1M"),
        ("duration", "P1M", "P1Y"),
        ("duration", "PT1S", "PT1M"),
        ("duration", "-P1Y", "P1Y"),
    ];
    for (name, small, large) in cases {
        let t = ty(name);
        assert_eq!(
            t.compare(small, large),
            Some(Less),
            "{name}: {small} < {large}"
        );
        assert_eq!(
            t.compare(large, small),
            Some(Greater),
            "{name}: {large} > {small}"
        );
        assert_eq!(
            t.compare(small, small),
            Some(std::cmp::Ordering::Equal),
            "{name}: {small} = {small}"
        );
    }
}

#[test]
fn an_unordered_type_has_no_comparison() {
    assert_eq!(ty("string").compare("a", "b"), None);
    assert_eq!(ty("boolean").compare("true", "false"), None);
    assert_eq!(ty("hexBinary").compare("00", "FF"), None);
}

#[test]
fn a_value_outside_its_type_has_no_place_in_the_ordering() {
    assert_eq!(ty("date").compare("not-a-date", "2001-01-01"), None);
    assert_eq!(ty("duration").compare("P", "P1Y"), None);
    assert_eq!(ty("integer").compare("1.5", "2"), None);
    assert_eq!(ty("double").compare("NaN", "1"), None);
}

#[test]
fn whitespace_is_processed_before_the_value_is_read() {
    // Every type but string and normalizedString collapses first, so
    // a padded value is still valid.
    assert!(ty("integer").accepts("  5  "));
    assert!(ty("boolean").accepts("\ttrue\n"));
    assert!(ty("date").accepts(" 2001-01-01 "));
    // And it is the collapsed value that gets compared.
    assert_eq!(
        ty("integer").compare(" 5 ", "5"),
        Some(std::cmp::Ordering::Equal)
    );
    // A string keeps its spaces, so they count towards its length.
    assert_eq!(ty("string").normalise(" a "), " a ");
}

#[test]
fn the_bounded_integers_reject_their_own_edges() {
    let cases: &[(&str, &str, &str)] = &[
        // (type, largest accepted, smallest rejected)
        ("byte", "127", "128"),
        ("short", "32767", "32768"),
        ("int", "2147483647", "2147483648"),
        ("long", "9223372036854775807", "9223372036854775808"),
        ("unsignedByte", "255", "256"),
        ("unsignedShort", "65535", "65536"),
        ("unsignedInt", "4294967295", "4294967296"),
        (
            "unsignedLong",
            "18446744073709551615",
            "18446744073709551616",
        ),
    ];
    for (name, ok, bad) in cases {
        assert!(ty(name).accepts(ok), "{name} should accept {ok}");
        assert!(!ty(name).accepts(bad), "{name} should reject {bad}");
    }
    // The unsigned types have a floor as well as a ceiling.
    for name in [
        "unsignedByte",
        "unsignedShort",
        "unsignedInt",
        "unsignedLong",
    ] {
        assert!(!ty(name).accepts("-1"), "{name} should reject -1");
    }
}

#[test]
fn the_list_types_require_every_item_to_be_valid() {
    assert!(ty("IDREFS").accepts("a b c"));
    assert!(!ty("IDREFS").accepts("a b:c"), "an IDREF is an NCName");
    assert!(ty("ENTITIES").accepts("one"));
    assert!(!ty("ENTITIES").accepts("1one"));
}
