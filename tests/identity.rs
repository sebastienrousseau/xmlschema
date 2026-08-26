// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 xmlschema. All rights reserved.

//! `xs:unique`, `xs:key` and `xs:keyref`.
//!
//! A selector chooses the nodes constrained and each field contributes
//! one component of a tuple. The three kinds differ in what they do
//! with a node whose fields are incomplete, and in whether the tuples
//! must be found elsewhere.

use xmlschema::{parse_schema, validate};

fn schema(body: &str) -> String {
    format!(
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">{body}</xs:schema>"#
    )
}

fn valid(body: &str, xml: &str) -> bool {
    let s = parse_schema(&schema(body)).expect("schema parses");
    let doc = oxml::parse(xml).expect("well-formed");
    validate(&doc, &s).is_valid()
}

const UNIQUE: &str = r#"
  <xs:element name="root">
    <xs:complexType><xs:sequence>
      <xs:element name="item" maxOccurs="unbounded">
        <xs:complexType><xs:attribute name="k" type="xs:string"/></xs:complexType>
      </xs:element>
    </xs:sequence></xs:complexType>
    <xs:unique name="u">
      <xs:selector xpath=".//item"/>
      <xs:field xpath="@k"/>
    </xs:unique>
  </xs:element>"#;

#[test]
fn unique_forbids_a_repeated_value() {
    assert!(valid(UNIQUE, r#"<root><item k="a"/><item k="b"/></root>"#));
    assert!(
        !valid(UNIQUE, r#"<root><item k="a"/><item k="a"/></root>"#),
        "`a` twice"
    );
    // A node missing the field is simply not constrained by `unique`.
    assert!(valid(UNIQUE, r#"<root><item k="a"/><item/></root>"#));
    assert!(valid(UNIQUE, "<root><item/><item/></root>"));
}

const KEY: &str = r#"
  <xs:element name="root">
    <xs:complexType><xs:sequence>
      <xs:element name="item" maxOccurs="unbounded">
        <xs:complexType><xs:attribute name="k" type="xs:string"/></xs:complexType>
      </xs:element>
    </xs:sequence></xs:complexType>
    <xs:key name="pk">
      <xs:selector xpath=".//item"/>
      <xs:field xpath="@k"/>
    </xs:key>
  </xs:element>"#;

#[test]
fn a_key_also_requires_the_field_to_be_present() {
    assert!(valid(KEY, r#"<root><item k="a"/><item k="b"/></root>"#));
    assert!(!valid(KEY, r#"<root><item k="a"/><item k="a"/></root>"#));
    // The difference from `unique`: a missing field is a violation.
    assert!(
        !valid(KEY, r#"<root><item k="a"/><item/></root>"#),
        "a key must be present on every selected node"
    );
}

const KEYREF: &str = r#"
  <xs:element name="root">
    <xs:complexType><xs:sequence>
      <xs:element name="item" maxOccurs="unbounded">
        <xs:complexType><xs:attribute name="k" type="xs:string"/></xs:complexType>
      </xs:element>
      <xs:element name="ref" minOccurs="0" maxOccurs="unbounded">
        <xs:complexType><xs:attribute name="to" type="xs:string"/></xs:complexType>
      </xs:element>
    </xs:sequence></xs:complexType>
    <xs:key name="pk">
      <xs:selector xpath=".//item"/>
      <xs:field xpath="@k"/>
    </xs:key>
    <xs:keyref name="fk" refer="pk">
      <xs:selector xpath=".//ref"/>
      <xs:field xpath="@to"/>
    </xs:keyref>
  </xs:element>"#;

#[test]
fn a_keyref_must_find_its_target() {
    assert!(valid(
        KEYREF,
        r#"<root><item k="a"/><item k="b"/><ref to="a"/></root>"#
    ));
    assert!(
        !valid(KEYREF, r#"<root><item k="a"/><ref to="z"/></root>"#),
        "`z` is not a key"
    );
    // No references at all is fine.
    assert!(valid(KEYREF, r#"<root><item k="a"/></root>"#));
}

/// A constraint over more than one field compares whole tuples.
#[test]
fn a_multi_field_constraint_compares_tuples() {
    let body = r#"
      <xs:element name="root">
        <xs:complexType><xs:sequence>
          <xs:element name="item" maxOccurs="unbounded">
            <xs:complexType>
              <xs:attribute name="a" type="xs:string"/>
              <xs:attribute name="b" type="xs:string"/>
            </xs:complexType>
          </xs:element>
        </xs:sequence></xs:complexType>
        <xs:unique name="u">
          <xs:selector xpath=".//item"/>
          <xs:field xpath="@a"/>
          <xs:field xpath="@b"/>
        </xs:unique>
      </xs:element>"#;
    // Sharing one component is not a duplicate.
    assert!(valid(
        body,
        r#"<root><item a="1" b="x"/><item a="1" b="y"/></root>"#
    ));
    assert!(valid(
        body,
        r#"<root><item a="1" b="x"/><item a="2" b="x"/></root>"#
    ));
    // Sharing both is.
    assert!(!valid(
        body,
        r#"<root><item a="1" b="x"/><item a="1" b="x"/></root>"#
    ));
}

/// The `id` attribute on a schema element is an `xs:ID`: an `NCName`,
/// and unique within the document.
#[test]
fn the_schema_id_attribute_is_an_xs_id() {
    assert!(parse_schema(&schema(r#"<xs:element name="a" id="ok"/>"#)).is_ok());
    // Empty is not an NCName.
    assert!(parse_schema(&schema(r#"<xs:element name="a" id=""/>"#)).is_err());
    // Nor is one with a colon, or a leading digit.
    assert!(
        parse_schema(&schema(r#"<xs:element name="a" id="a:b"/>"#)).is_err()
    );
    assert!(
        parse_schema(&schema(r#"<xs:element name="a" id="1x"/>"#)).is_err()
    );
    // And no two may share a value.
    assert!(
        parse_schema(&schema(
            r#"<xs:element name="a" id="dup"/><xs:element name="b" id="dup"/>"#
        ))
        .is_err()
    );
}
