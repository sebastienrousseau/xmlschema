// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 xmlschema. All rights reserved.

//! `xs:import` and `xs:include`, resolved by the caller.
//!
//! Neither this crate nor `oxml` performs I/O. A schema referencing
//! another names a location; resolving it is a policy decision, and a
//! validator that fetches by default is how one becomes an outbound
//! request.

use xmlschema::{SchemaSource, parse_schema, parse_schema_with, validate};

const COMMON: &str = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:simpleType name="code">
    <xs:restriction base="xs:string">
      <xs:maxLength value="4"/>
    </xs:restriction>
  </xs:simpleType>
</xs:schema>"#;

const MAIN: &str = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:include schemaLocation="common.xsd"/>
  <xs:element name="r" type="code"/>
</xs:schema>"#;

#[test]
fn an_included_type_is_enforced() {
    let parts: &[(&str, &str)] = &[("common.xsd", COMMON)];
    let schema = parse_schema_with(MAIN, &parts).expect("schema parses");

    let ok = oxml::parse("<r>abcd</r>").expect("well-formed");
    assert!(validate(&ok, &schema).is_valid());

    // The included facet applies, which is the whole point.
    let long = oxml::parse("<r>abcde</r>").expect("well-formed");
    assert!(!validate(&long, &schema).is_valid());
}

#[test]
fn without_a_source_nothing_is_resolved_and_nothing_is_fetched() {
    let schema = parse_schema(MAIN).expect("still a valid schema");
    // `code` never resolved, so the element is unconstrained rather
    // than the schema being rejected.
    let anything =
        oxml::parse("<r>far longer than four</r>").expect("well-formed");
    assert!(validate(&anything, &schema).is_valid());

    // And the gap is reported rather than silent. It surfaces as the
    // unresolved *type*, not as the include: `support::unsupported`
    // reads the document alone and cannot know whether a source would
    // have supplied `common.xsd`. What it can see is that `code`
    // resolves to nothing, which is the effect that matters.
    let doc = oxml::parse(MAIN).expect("well-formed");
    let gaps = xmlschema::support::unsupported(&doc);
    assert!(
        gaps.iter().any(|g| g.construct.contains("code")),
        "the unresolved reference must be reported: {gaps:?}"
    );
}

#[test]
fn a_location_the_source_declines_is_not_an_error() {
    // An empty source supplies nothing; the schema is still valid.
    let empty: &[(&str, &str)] = &[];
    assert!(parse_schema_with(MAIN, &empty).is_ok());
}

#[test]
fn a_reference_chain_is_followed() {
    let leaf = COMMON;
    let middle = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
      <xs:include schemaLocation="common.xsd"/>
    </xs:schema>"#;
    let parts: &[(&str, &str)] =
        &[("common.xsd", leaf), ("middle.xsd", middle)];
    let top = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
      <xs:include schemaLocation="middle.xsd"/>
      <xs:element name="r" type="code"/>
    </xs:schema>"#;
    let schema = parse_schema_with(top, &parts).expect("schema parses");
    let long = oxml::parse("<r>abcde</r>").expect("well-formed");
    assert!(!validate(&long, &schema).is_valid(), "resolved two deep");
}

/// A pair of schemas that reference one another must terminate.
#[test]
fn a_cycle_is_bounded_rather_than_followed_forever() {
    let a = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
      <xs:include schemaLocation="b.xsd"/>
      <xs:element name="r" type="xs:string"/>
    </xs:schema>"#;
    let b = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
      <xs:include schemaLocation="a.xsd"/>
    </xs:schema>"#;
    let parts: &[(&str, &str)] = &[("a.xsd", a), ("b.xsd", b)];
    // Must return rather than recurse until the stack runs out.
    assert!(parse_schema_with(a, &parts).is_ok());
}

#[test]
fn a_supplied_document_that_is_not_a_schema_is_reported() {
    let parts: &[(&str, &str)] = &[("common.xsd", "<notSchema/>")];
    assert!(parse_schema_with(MAIN, &parts).is_err());
}

#[test]
fn a_slice_of_pairs_is_a_source() {
    let parts: &[(&str, &str)] = &[("a.xsd", "<a/>"), ("b.xsd", "<b/>")];
    assert_eq!(parts.fetch("a.xsd"), Some("<a/>"));
    assert_eq!(parts.fetch("b.xsd"), Some("<b/>"));
    assert_eq!(parts.fetch("missing.xsd"), None);
}

/// A referenced document contributes complex types as well as simple
/// ones, and elements.
#[test]
fn every_kind_of_declaration_is_contributed() {
    let common = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
      <xs:complexType name="pair">
        <xs:sequence>
          <xs:element name="a" type="xs:integer"/>
          <xs:element name="b" type="xs:integer"/>
        </xs:sequence>
      </xs:complexType>
      <xs:element name="shared" type="xs:integer"/>
    </xs:schema>"#;
    let main = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
      <xs:include schemaLocation="common.xsd"/>
      <xs:element name="r" type="pair"/>
    </xs:schema>"#;
    let parts: &[(&str, &str)] = &[("common.xsd", common)];
    let schema = parse_schema_with(main, &parts).expect("schema parses");

    // The included complex type is applied.
    let ok = oxml::parse("<r><a>1</a><b>2</b></r>").expect("well-formed");
    assert!(validate(&ok, &schema).is_valid());
    let bad = oxml::parse("<r><a>x</a><b>2</b></r>").expect("well-formed");
    assert!(!validate(&bad, &schema).is_valid());

    // And the included element declaration is available.
    assert!(schema.element("shared").is_some());
}

/// A local declaration wins over one of the same name from elsewhere.
#[test]
fn a_local_declaration_takes_precedence() {
    let common = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
      <xs:simpleType name="code">
        <xs:restriction base="xs:string"><xs:maxLength value="1"/></xs:restriction>
      </xs:simpleType>
    </xs:schema>"#;
    let main = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
      <xs:include schemaLocation="common.xsd"/>
      <xs:simpleType name="code">
        <xs:restriction base="xs:string"><xs:maxLength value="8"/></xs:restriction>
      </xs:simpleType>
      <xs:element name="r" type="code"/>
    </xs:schema>"#;
    let parts: &[(&str, &str)] = &[("common.xsd", common)];
    let schema = parse_schema_with(main, &parts).expect("schema parses");
    // The local `code` allows eight characters, the included one only
    // a single character.
    let doc = oxml::parse("<r>abcdefg</r>").expect("well-formed");
    assert!(validate(&doc, &schema).is_valid(), "the local type wins");
}

/// `xs:import` is followed the same way `xs:include` is.
#[test]
fn import_is_resolved_as_include_is() {
    let other = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
      <xs:simpleType name="code">
        <xs:restriction base="xs:string"><xs:maxLength value="2"/></xs:restriction>
      </xs:simpleType>
    </xs:schema>"#;
    let main = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
      <xs:import namespace="urn:other" schemaLocation="other.xsd"/>
      <xs:element name="r" type="code"/>
    </xs:schema>"#;
    let parts: &[(&str, &str)] = &[("other.xsd", other)];
    let schema = parse_schema_with(main, &parts).expect("schema parses");
    let long = oxml::parse("<r>abc</r>").expect("well-formed");
    assert!(!validate(&long, &schema).is_valid());
}

/// A reference with no `schemaLocation` names nothing to fetch.
#[test]
fn a_reference_without_a_location_is_skipped() {
    let main = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
      <xs:import namespace="urn:other"/>
      <xs:element name="r" type="xs:string"/>
    </xs:schema>"#;
    let parts: &[(&str, &str)] = &[];
    assert!(parse_schema_with(main, &parts).is_ok());
}
