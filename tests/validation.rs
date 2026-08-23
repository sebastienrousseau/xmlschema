// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 xmlschema. All rights reserved.

//! Validation behaviour against real schemas.

use xmlschema::{Report, parse_schema, validate};

const LIBRARY_XSD: &str = r#"
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:simpleType name="isbn">
    <xs:restriction base="xs:string">
      <xs:pattern value="\d{3}-\d{10}"/>
    </xs:restriction>
  </xs:simpleType>

  <xs:simpleType name="lang">
    <xs:restriction base="xs:string">
      <xs:enumeration value="en"/>
      <xs:enumeration value="fr"/>
      <xs:enumeration value="de"/>
    </xs:restriction>
  </xs:simpleType>

  <xs:element name="library">
    <xs:complexType>
      <xs:sequence>
        <xs:element name="book" minOccurs="1" maxOccurs="unbounded">
          <xs:complexType>
            <xs:sequence>
              <xs:element name="title" type="xs:string"/>
              <xs:element name="year">
                <xs:simpleType>
                  <xs:restriction base="xs:integer">
                    <xs:minInclusive value="1450"/>
                    <xs:maxInclusive value="2100"/>
                  </xs:restriction>
                </xs:simpleType>
              </xs:element>
            </xs:sequence>
            <xs:attribute name="lang" type="lang" use="required"/>
            <xs:attribute name="isbn" type="isbn"/>
          </xs:complexType>
        </xs:element>
      </xs:sequence>
    </xs:complexType>
  </xs:element>
</xs:schema>
"#;

fn check(xml: &str) -> Report {
    let schema = parse_schema(LIBRARY_XSD).expect("schema parses");
    let doc = oxml::parse(xml).expect("document parses");
    validate(&doc, &schema)
}

#[test]
fn a_conforming_document_is_valid() {
    let report = check(
        r#"<library>
             <book lang="en" isbn="978-0441013593">
               <title>Dune</title><year>1965</year>
             </book>
           </library>"#,
    );
    assert!(report.is_valid(), "{report}");
}

#[test]
fn repeated_elements_within_bounds_are_valid() {
    let report = check(
        r#"<library>
             <book lang="en"><title>A</title><year>2000</year></book>
             <book lang="fr"><title>B</title><year>2001</year></book>
             <book lang="de"><title>C</title><year>2002</year></book>
           </library>"#,
    );
    assert!(report.is_valid(), "{report}");
}

#[test]
fn a_missing_required_attribute_is_reported_with_its_path() {
    let report = check(
        r"<library><book><title>A</title><year>2000</year></book></library>",
    );
    assert_eq!(report.violations.len(), 1);
    assert_eq!(report.violations[0].path, "/library/book[1]");
    assert!(
        report.violations[0].message.contains("lang"),
        "{}",
        report.violations[0].message
    );
}

#[test]
fn an_enumeration_violation_names_the_permitted_values() {
    let report = check(
        r#"<library><book lang="es"><title>A</title><year>2000</year></book></library>"#,
    );
    assert_eq!(report.violations.len(), 1);
    assert_eq!(report.violations[0].path, "/library/book[1]/@lang");
    let m = &report.violations[0].message;
    assert!(m.contains("en") && m.contains("fr"), "{m}");
}

#[test]
fn a_pattern_violation_is_reported() {
    let report = check(
        r#"<library><book lang="en" isbn="nope">
             <title>A</title><year>2000</year></book></library>"#,
    );
    assert_eq!(report.violations.len(), 1);
    assert!(
        report.violations[0].message.contains("pattern"),
        "{}",
        report.violations[0].message
    );
}

#[test]
fn numeric_bounds_are_enforced() {
    let too_old = check(
        r#"<library><book lang="en"><title>A</title><year>1200</year></book></library>"#,
    );
    assert_eq!(too_old.violations.len(), 1);
    assert!(too_old.violations[0].message.contains("1450"));

    let ok = check(
        r#"<library><book lang="en"><title>A</title><year>1450</year></book></library>"#,
    );
    assert!(ok.is_valid(), "{ok}");
}

#[test]
fn a_non_integer_where_an_integer_is_required_is_reported() {
    let report = check(
        r#"<library><book lang="en"><title>A</title><year>MCMLXV</year></book></library>"#,
    );
    assert_eq!(report.violations.len(), 1);
    assert!(
        report.violations[0].message.contains("integer"),
        "{}",
        report.violations[0].message
    );
}

#[test]
fn missing_children_report_the_expected_cardinality() {
    let report = check(r#"<library><book lang="en"></book></library>"#);
    let messages: Vec<&str> = report
        .violations
        .iter()
        .map(|v| v.message.as_str())
        .collect();
    assert!(messages.iter().any(|m| m.contains("title")), "{messages:?}");
    assert!(messages.iter().any(|m| m.contains("year")), "{messages:?}");
}

#[test]
fn an_unexpected_element_names_what_was_allowed() {
    let report = check(
        r#"<library><book lang="en"><title>A</title><year>2000</year>
             <publisher>X</publisher></book></library>"#,
    );
    assert_eq!(report.violations.len(), 1);
    let m = &report.violations[0].message;
    assert!(m.contains("publisher") && m.contains("title"), "{m}");
}

/// Elements out of order are an *ordering* problem, and the message
/// should not blame cardinality for it.
#[test]
fn out_of_order_children_are_reported() {
    let report = check(
        r#"<library><book lang="en"><year>2000</year><title>A</title></book></library>"#,
    );
    assert!(!report.is_valid());
}

#[test]
fn every_violation_is_reported_not_just_the_first() {
    let report = check(
        r#"<library>
             <book lang="es"><title>A</title><year>nope</year></book>
             <book><title>B</title><year>1200</year></book>
           </library>"#,
    );
    // bad lang, bad year, missing lang, out-of-range year.
    assert!(
        report.violations.len() >= 4,
        "expected at least 4, got {}: {report}",
        report.violations.len()
    );
}

#[test]
fn an_unknown_root_element_lists_what_the_schema_declares() {
    let schema = parse_schema(LIBRARY_XSD).expect("schema parses");
    let doc = oxml::parse("<catalogue/>").expect("parses");
    let report = validate(&doc, &schema);
    assert_eq!(report.violations.len(), 1);
    assert!(report.violations[0].message.contains("library"));
}

#[test]
fn a_malformed_schema_is_rejected_with_a_reason() {
    let err = parse_schema("<xs:schema>").expect_err("not well-formed");
    assert!(err.message.contains("well-formed"), "{}", err.message);

    let err = parse_schema("<root/>").expect_err("not a schema");
    assert!(err.message.contains("xs:schema"), "{}", err.message);
}

#[test]
fn choice_accepts_any_declared_branch() {
    let xsd = r#"
      <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
        <xs:element name="payment">
          <xs:complexType>
            <xs:choice>
              <xs:element name="card" type="xs:string"/>
              <xs:element name="cash" type="xs:string"/>
            </xs:choice>
          </xs:complexType>
        </xs:element>
      </xs:schema>"#;
    let schema = parse_schema(xsd).expect("schema parses");

    for xml in [
        "<payment><card>x</card></payment>",
        "<payment><cash>y</cash></payment>",
    ] {
        let doc = oxml::parse(xml).expect("parses");
        assert!(validate(&doc, &schema).is_valid(), "{xml}");
    }

    let doc =
        oxml::parse("<payment><cheque>z</cheque></payment>").expect("parses");
    let report = validate(&doc, &schema);
    assert_eq!(report.violations.len(), 1);
    assert!(report.violations[0].message.contains("card"));
}
