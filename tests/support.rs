// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 xmlschema. All rights reserved.

//! What a schema stops this crate from enforcing.
//!
//! This module decides whether a conformance figure means anything: a
//! schema whose constraints were skipped accepts every document, so an
//! agreement with a test suite proves nothing. Every case here is one
//! where "the answer happened to be right" and "the constraint was
//! enforced" come apart.

use xmlschema::support::unsupported;

/// Wrap a schema body in an `xs:schema` element.
fn schema(body: &str) -> String {
    format!(
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">{body}</xs:schema>"#
    )
}

fn gaps(body: &str) -> Vec<String> {
    let doc = oxml::parse(&schema(body)).expect("well-formed");
    unsupported(&doc).into_iter().map(|u| u.construct).collect()
}

#[test]
fn a_fully_supported_schema_reports_nothing() {
    let found = gaps(
        r#"<xs:element name="r">
             <xs:complexType>
               <xs:sequence>
                 <xs:element name="a" type="xs:string"/>
                 <xs:element name="n" type="xs:integer"/>
               </xs:sequence>
               <xs:attribute name="k" type="xs:date" use="required"/>
             </xs:complexType>
           </xs:element>"#,
    );
    assert!(
        found.is_empty(),
        "nothing should be unenforceable: {found:?}"
    );
}

#[test]
fn an_unhandled_element_is_reported() {
    for (body, want) in [
        (r#"<xs:import namespace="urn:x"/>"#, "xs:import"),
        (r#"<xs:include schemaLocation="a.xsd"/>"#, "xs:include"),
        (r#"<xs:notation name="n" public="p"/>"#, "xs:notation"),
        (
            r#"<xs:element name="r"><xs:complexType><xs:sequence>
                 <xs:element name="a" type="xs:string"/>
               </xs:sequence></xs:complexType>
               <xs:redefine schemaLocation="a.xsd"/>
             </xs:element>"#,
            "xs:redefine",
        ),
    ] {
        let found = gaps(body);
        assert!(
            found.iter().any(|c| c == want),
            "expected {want} in {found:?}"
        );
    }
}

#[test]
fn an_unresolvable_type_reference_is_reported() {
    let found = gaps(r#"<xs:element name="r" type="other:Thing"/>"#);
    assert!(
        found.iter().any(|c| c.contains("other:Thing")),
        "an unresolved type constrains nothing: {found:?}"
    );
}

#[test]
fn a_type_declared_in_the_same_schema_is_not_a_gap() {
    let found = gaps(
        r#"<xs:simpleType name="code">
             <xs:restriction base="xs:string"><xs:maxLength value="4"/></xs:restriction>
           </xs:simpleType>
           <xs:element name="r" type="code"/>"#,
    );
    assert!(found.is_empty(), "a local type resolves: {found:?}");
}

#[test]
fn every_built_in_resolves_without_a_gap() {
    // The lattice is modelled in full, so naming any built-in is
    // enforceable. This used to report a dozen of them as lossy.
    for ty in [
        "string",
        "boolean",
        "decimal",
        "integer",
        "byte",
        "int",
        "long",
        "short",
        "unsignedByte",
        "positiveInteger",
        "negativeInteger",
        "float",
        "double",
        "date",
        "dateTime",
        "time",
        "duration",
        "gYear",
        "gMonth",
        "gDay",
        "gYearMonth",
        "gMonthDay",
        "hexBinary",
        "base64Binary",
        "anyURI",
        "QName",
        "NCName",
        "Name",
        "NMTOKEN",
        "NMTOKENS",
        "ID",
        "IDREF",
        "IDREFS",
        "language",
        "token",
        "normalizedString",
    ] {
        let found = gaps(&format!(r#"<xs:element name="r" type="xs:{ty}"/>"#));
        assert!(found.is_empty(), "xs:{ty} should be enforceable: {found:?}");
    }
}

#[test]
fn an_attribute_that_changes_meaning_is_reported() {
    for (body, want) in [
        (
            r#"<xs:element name="r" abstract="true"/>"#,
            "@abstract on xs:element",
        ),
        (
            r#"<xs:element name="r" substitutionGroup="other"/>"#,
            "@substitutionGroup on xs:element",
        ),
        (
            r#"<xs:element name="r" block="extension"/>"#,
            "@block on xs:element",
        ),
        (
            r#"<xs:element name="r" final="restriction"/>"#,
            "@final on xs:element",
        ),
        (
            r#"<xs:element name="r" type="xs:string" default="x"/>"#,
            "@default on xs:element",
        ),
    ] {
        let found = gaps(body);
        assert!(
            found.iter().any(|c| c == want),
            "expected {want} in {found:?}"
        );
    }
}

/// A pattern the engine cannot compile constrains nothing, and saying
/// so is what keeps it from being blamed on the document.
#[test]
fn an_uncompilable_pattern_is_reported() {
    let found = gaps(
        r#"<xs:element name="r">
             <xs:simpleType><xs:restriction base="xs:string">
               <xs:pattern value="\p{IsBasicLatin}+"/>
             </xs:restriction></xs:simpleType>
           </xs:element>"#,
    );
    assert!(
        found.iter().any(|c| c.starts_with("xs:pattern")),
        "an uncompilable pattern is a gap: {found:?}"
    );
}

#[test]
fn a_compilable_pattern_is_not_reported() {
    let found = gaps(
        r#"<xs:element name="r">
             <xs:simpleType><xs:restriction base="xs:string">
               <xs:pattern value="[a-z]{2,4}"/>
             </xs:restriction></xs:simpleType>
           </xs:element>"#,
    );
    assert!(found.is_empty(), "this pattern compiles: {found:?}");
}

/// The report carries what stops being checked, not only what was
/// found — a bare construct name does not tell a reader what they lose.
#[test]
fn each_gap_explains_what_stops_being_checked() {
    let doc = oxml::parse(&schema(r#"<xs:import namespace="urn:x"/>"#))
        .expect("well-formed");
    let found = unsupported(&doc);
    assert!(!found.is_empty());
    for gap in found {
        assert!(!gap.construct.is_empty());
        assert!(
            !gap.effect.is_empty(),
            "`{}` reports no effect",
            gap.construct
        );
    }
}

#[test]
fn gaps_are_deduplicated_and_ordered() {
    // The same construct used many times is one gap, not many.
    let found = gaps(
        r#"<xs:element name="a" abstract="true"/>
           <xs:element name="b" abstract="true"/>
           <xs:element name="c" abstract="true"/>"#,
    );
    assert_eq!(found.len(), 1, "one distinct gap: {found:?}");

    let mut sorted = found.clone();
    sorted.sort();
    assert_eq!(found, sorted, "gaps come back in a stable order");
}
