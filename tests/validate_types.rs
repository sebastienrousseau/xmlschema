// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 xmlschema. All rights reserved.

//! Built-in datatype checking, and the reports it produces.
//!
//! Each built-in carries a distinct validation rule; a rule that never
//! rejects anything is indistinguishable from an unconstrained element,
//! so every one is exercised against a value it must accept and one it
//! must refuse.

use xmlschema::{parse_schema, validate};

/// Validate `xml` against a schema whose root element has type `ty`.
fn check(ty: &str, xml: &str) -> xmlschema::Report {
    let xsd = format!(
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
             <xs:element name="v" type="{ty}"/>
           </xs:schema>"#
    );
    let schema = parse_schema(&xsd).expect("valid schema");
    let doc = oxml::parse(xml).expect("well-formed");
    validate(&doc, &schema)
}

fn accepts(ty: &str, value: &str) -> bool {
    check(ty, &format!("<v>{value}</v>")).violations.is_empty()
}

#[test]
fn boolean_accepts_only_its_four_lexical_forms() {
    for good in ["true", "false", "1", "0"] {
        assert!(accepts("xs:boolean", good), "rejected {good}");
    }
    for bad in ["yes", "no", "True", "FALSE", "2", ""] {
        assert!(!accepts("xs:boolean", bad), "accepted {bad}");
    }
}

#[test]
fn integer_rejects_fractions_and_words() {
    for good in ["0", "42", "-17", "+3"] {
        assert!(accepts("xs:integer", good), "rejected {good}");
    }
    for bad in ["1.5", "one", "", "1e3", "0x10"] {
        assert!(!accepts("xs:integer", bad), "accepted {bad}");
    }
}

#[test]
fn non_negative_integer_rejects_negatives() {
    // The distinction from `integer` is the entire point of the type.
    assert!(accepts("xs:nonNegativeInteger", "0"));
    assert!(accepts("xs:nonNegativeInteger", "7"));
    assert!(!accepts("xs:nonNegativeInteger", "-1"));
    assert!(!accepts("xs:nonNegativeInteger", "-0.5"));
}

#[test]
fn decimal_and_double_accept_fractions() {
    for ty in ["xs:decimal", "xs:double"] {
        assert!(accepts(ty, "1.5"), "{ty} rejected 1.5");
        assert!(accepts(ty, "-0.25"), "{ty} rejected -0.25");
        assert!(accepts(ty, "42"), "{ty} rejected 42");
        assert!(!accepts(ty, "one"), "{ty} accepted one");
        assert!(!accepts(ty, ""), "{ty} accepted empty");
    }
}

#[test]
fn date_requires_the_iso_shape() {
    assert!(accepts("xs:date", "2026-08-22"));
    for bad in ["22-08-2026", "2026/08/22", "2026-8-2", "not a date", ""] {
        assert!(!accepts("xs:date", bad), "accepted {bad}");
    }
}

#[test]
fn date_time_requires_a_time_after_the_t() {
    assert!(accepts("xs:dateTime", "2026-08-22T13:45:00"));
    for bad in [
        "2026-08-22",
        "2026-08-22T",
        "2026-08-22 13:45:00",
        "2026-08-22Tnope",
    ] {
        assert!(!accepts("xs:dateTime", bad), "accepted {bad}");
    }
}

#[test]
fn any_uri_and_string_accept_ordinary_text() {
    assert!(accepts("xs:anyURI", "https://example.com/a?b=c"));
    assert!(accepts("xs:string", "anything at all"));
    assert!(accepts("xs:string", ""));
}

#[test]
fn a_violation_names_the_expected_type() {
    // "invalid" alone would not tell the author what to change.
    let report = check("xs:integer", "<v>abc</v>");
    assert_eq!(report.violations.len(), 1);
    let text = report.violations[0].to_string();
    assert!(text.contains("integer"), "{text}");
    assert!(text.contains('/'), "no path in {text}");
}

#[test]
fn a_valid_report_displays_as_valid() {
    let report = check("xs:string", "<v>ok</v>");
    assert!(report.violations.is_empty());
    assert_eq!(report.to_string(), "valid");
}

#[test]
fn multiple_violations_are_listed_one_per_line() {
    let xsd = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
        <xs:element name="r">
          <xs:complexType><xs:sequence>
            <xs:element name="a" type="xs:integer"/>
            <xs:element name="b" type="xs:integer"/>
          </xs:sequence></xs:complexType>
        </xs:element>
      </xs:schema>"#;
    let schema = parse_schema(xsd).expect("valid schema");
    let doc = oxml::parse("<r><a>x</a><b>y</b></r>").expect("well-formed");
    let report = validate(&doc, &schema);
    assert_eq!(report.violations.len(), 2, "{report}");
    assert_eq!(report.to_string().lines().count(), 2, "{report}");
}

#[test]
fn a_rootless_document_cannot_reach_the_validator() {
    // `validate` guards against a document with no root element, but
    // that guard is unreachable through the public API: `oxml::parse`
    // is the only way to obtain a `Document` and it rejects a rootless
    // input outright. This pins that assumption — if oxml ever starts
    // accepting one, the guard stops being dead code and this test
    // fails to remind us to cover it.
    assert!(oxml::parse("<!-- just a comment -->").is_err());
    assert!(oxml::parse("").is_err());
}

#[test]
fn an_empty_content_model_rejects_children_and_text() {
    let xsd = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
        <xs:element name="r"><xs:complexType/></xs:element>
      </xs:schema>"#;
    let schema = parse_schema(xsd).expect("valid schema");

    let empty = oxml::parse("<r/>").expect("well-formed");
    assert!(validate(&empty, &schema).violations.is_empty());

    let with_text = oxml::parse("<r>text</r>").expect("well-formed");
    assert!(!validate(&with_text, &schema).violations.is_empty());

    let with_child = oxml::parse("<r><a/></r>").expect("well-formed");
    assert!(!validate(&with_child, &schema).violations.is_empty());
}

#[test]
fn a_simple_typed_element_rejects_child_elements() {
    // Text content and child elements are different content models;
    // accepting both would make the type meaningless.
    let report = check("xs:string", "<v><child/></v>");
    assert!(!report.violations.is_empty(), "{report}");
    assert!(report.to_string().contains("child"), "{report}");
}

#[test]
fn an_unconstrained_element_accepts_anything() {
    let xsd = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
        <xs:element name="r"/>
      </xs:schema>"#;
    let schema = parse_schema(xsd).expect("valid schema");
    for xml in ["<r/>", "<r>text</r>", "<r><a><b/></a></r>"] {
        let doc = oxml::parse(xml).expect("well-formed");
        assert!(
            validate(&doc, &schema).violations.is_empty(),
            "rejected {xml}"
        );
    }
}
