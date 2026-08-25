// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 xmlschema. All rights reserved.

//! References, groups, derivation and the content models built on them.
//!
//! These constructs share a root: they only mean anything once a
//! top-level declaration can be looked up. Each was previously skipped,
//! which left the declaration using it unconstrained — and an
//! unconstrained declaration agrees with every document, so nothing
//! failed.

use std::fmt::Write as _;

use xmlschema::{parse_schema, validate};

fn schema(body: &str) -> String {
    format!(
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">{body}</xs:schema>"#
    )
}

/// Whether `xml` validates against a schema built from `body`.
fn valid(body: &str, xml: &str) -> bool {
    let s = parse_schema(&schema(body)).expect("schema parses");
    let doc = oxml::parse(xml).expect("well-formed");
    validate(&doc, &s).is_valid()
}

#[test]
fn an_element_ref_reuses_the_top_level_declaration() {
    let body = r#"
      <xs:element name="shared" type="xs:integer"/>
      <xs:element name="r">
        <xs:complexType><xs:sequence>
          <xs:element ref="shared"/>
        </xs:sequence></xs:complexType>
      </xs:element>"#;
    assert!(valid(body, "<r><shared>42</shared></r>"));
    // The referenced type is enforced, which is the whole point.
    assert!(!valid(body, "<r><shared>not a number</shared></r>"));
    assert!(!valid(body, "<r></r>"), "still required");
}

#[test]
fn a_ref_carries_its_own_cardinality() {
    let body = r#"
      <xs:element name="item" type="xs:string"/>
      <xs:element name="r">
        <xs:complexType><xs:sequence>
          <xs:element ref="item" minOccurs="0" maxOccurs="2"/>
        </xs:sequence></xs:complexType>
      </xs:element>"#;
    assert!(valid(body, "<r></r>"));
    assert!(valid(body, "<r><item>a</item><item>b</item></r>"));
    assert!(
        !valid(body, "<r><item>a</item><item>b</item><item>c</item></r>"),
        "three exceeds maxOccurs"
    );
}

#[test]
fn a_named_group_is_spliced_in() {
    let body = r#"
      <xs:group name="pair">
        <xs:sequence>
          <xs:element name="a" type="xs:string"/>
          <xs:element name="b" type="xs:integer"/>
        </xs:sequence>
      </xs:group>
      <xs:element name="r">
        <xs:complexType><xs:sequence>
          <xs:group ref="pair"/>
        </xs:sequence></xs:complexType>
      </xs:element>"#;
    assert!(valid(body, "<r><a>x</a><b>1</b></r>"));
    assert!(!valid(body, "<r><a>x</a><b>no</b></r>"), "b is an integer");
    assert!(!valid(body, "<r><a>x</a></r>"), "b is required");
}

#[test]
fn a_named_attribute_group_is_spliced_in() {
    let body = r#"
      <xs:attributeGroup name="common">
        <xs:attribute name="id" type="xs:integer" use="required"/>
        <xs:attribute name="note" type="xs:string"/>
      </xs:attributeGroup>
      <xs:element name="r">
        <xs:complexType>
          <xs:attributeGroup ref="common"/>
        </xs:complexType>
      </xs:element>"#;
    assert!(valid(body, r#"<r id="1"/>"#));
    assert!(valid(body, r#"<r id="1" note="x"/>"#));
    assert!(!valid(body, "<r/>"), "id is required");
    assert!(!valid(body, r#"<r id="x"/>"#), "id is an integer");
}

#[test]
fn an_attribute_ref_reuses_the_top_level_declaration() {
    let body = r#"
      <xs:attribute name="when" type="xs:date"/>
      <xs:element name="r">
        <xs:complexType>
          <xs:attribute ref="when" use="required"/>
        </xs:complexType>
      </xs:element>"#;
    assert!(valid(body, r#"<r when="2001-01-01"/>"#));
    assert!(!valid(body, r#"<r when="nonsense"/>"#));
    assert!(!valid(body, "<r/>"), "the use here says required");
}

#[test]
fn complex_content_extension_appends_to_the_base() {
    let body = r#"
      <xs:complexType name="base">
        <xs:sequence><xs:element name="a" type="xs:string"/></xs:sequence>
      </xs:complexType>
      <xs:complexType name="derived">
        <xs:complexContent>
          <xs:extension base="base">
            <xs:sequence><xs:element name="b" type="xs:integer"/></xs:sequence>
          </xs:extension>
        </xs:complexContent>
      </xs:complexType>
      <xs:element name="r" type="derived"/>"#;
    assert!(
        valid(body, "<r><a>x</a><b>1</b></r>"),
        "base then extension"
    );
    assert!(
        !valid(body, "<r><a>x</a></r>"),
        "b comes with the extension"
    );
    assert!(!valid(body, "<r><b>1</b></r>"), "a comes from the base");
}

#[test]
fn complex_content_restriction_states_its_model_in_full() {
    let body = r#"
      <xs:complexType name="base">
        <xs:sequence>
          <xs:element name="a" type="xs:string"/>
          <xs:element name="b" type="xs:string" minOccurs="0"/>
        </xs:sequence>
      </xs:complexType>
      <xs:complexType name="narrow">
        <xs:complexContent>
          <xs:restriction base="base">
            <xs:sequence><xs:element name="a" type="xs:string"/></xs:sequence>
          </xs:restriction>
        </xs:complexContent>
      </xs:complexType>
      <xs:element name="r" type="narrow"/>"#;
    assert!(valid(body, "<r><a>x</a></r>"));
    assert!(
        !valid(body, "<r><a>x</a><b>y</b></r>"),
        "b is restricted away"
    );
}

#[test]
fn attributes_are_inherited_through_an_extension() {
    let body = r#"
      <xs:complexType name="base">
        <xs:attribute name="id" type="xs:integer" use="required"/>
      </xs:complexType>
      <xs:complexType name="derived">
        <xs:complexContent>
          <xs:extension base="base">
            <xs:attribute name="extra" type="xs:string"/>
          </xs:extension>
        </xs:complexContent>
      </xs:complexType>
      <xs:element name="r" type="derived"/>"#;
    assert!(valid(body, r#"<r id="1" extra="x"/>"#));
    assert!(
        !valid(body, r#"<r extra="x"/>"#),
        "id is inherited and required"
    );
    assert!(!valid(body, r#"<r id="no"/>"#), "and still an integer");
}

#[test]
fn simple_content_extension_keeps_the_base_type() {
    let body = r#"
      <xs:element name="r">
        <xs:complexType>
          <xs:simpleContent>
            <xs:extension base="xs:integer">
              <xs:attribute name="unit" type="xs:string"/>
            </xs:extension>
          </xs:simpleContent>
        </xs:complexType>
      </xs:element>"#;
    assert!(valid(body, r#"<r unit="m">42</r>"#));
    assert!(
        !valid(body, r#"<r unit="m">wide</r>"#),
        "the content is integer"
    );
}

#[test]
fn a_fixed_element_value_is_enforced() {
    let body = r#"<xs:element name="r" type="xs:string" fixed="only"/>"#;
    assert!(valid(body, "<r>only</r>"));
    assert!(!valid(body, "<r>other</r>"));
}

#[test]
fn a_prohibited_attribute_is_rejected() {
    let body = r#"
      <xs:element name="r">
        <xs:complexType>
          <xs:attribute name="gone" type="xs:string" use="prohibited"/>
        </xs:complexType>
      </xs:element>"#;
    assert!(valid(body, "<r/>"));
    assert!(!valid(body, r#"<r gone="x"/>"#));
}

#[test]
fn xsi_nil_stands_in_for_content_only_where_permitted() {
    const NS: &str = r#"xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance""#;
    let nillable =
        r#"<xs:element name="r" type="xs:integer" nillable="true"/>"#;
    let plain = r#"<xs:element name="r" type="xs:integer"/>"#;

    assert!(valid(nillable, &format!(r#"<r {NS} xsi:nil="true"/>"#)));
    assert!(
        !valid(plain, &format!(r#"<r {NS} xsi:nil="true"/>"#)),
        "not nillable"
    );
    assert!(
        !valid(nillable, &format!(r#"<r {NS} xsi:nil="true">1</r>"#)),
        "a nilled element must be empty"
    );
}

#[test]
fn a_union_accepts_any_member() {
    let body = r#"
      <xs:element name="r">
        <xs:simpleType>
          <xs:union memberTypes="xs:integer xs:date"/>
        </xs:simpleType>
      </xs:element>"#;
    assert!(valid(body, "<r>42</r>"));
    assert!(valid(body, "<r>2001-01-01</r>"));
    assert!(!valid(body, "<r>neither</r>"));
}

#[test]
fn a_union_may_nest_its_members() {
    let body = r#"
      <xs:element name="r">
        <xs:simpleType>
          <xs:union>
            <xs:simpleType>
              <xs:restriction base="xs:string">
                <xs:enumeration value="yes"/>
              </xs:restriction>
            </xs:simpleType>
            <xs:simpleType><xs:restriction base="xs:integer"/></xs:simpleType>
          </xs:union>
        </xs:simpleType>
      </xs:element>"#;
    assert!(valid(body, "<r>yes</r>"));
    assert!(valid(body, "<r>7</r>"));
    assert!(!valid(body, "<r>no</r>"));
}

#[test]
fn a_list_validates_every_item() {
    let body = r#"
      <xs:element name="r">
        <xs:simpleType><xs:list itemType="xs:integer"/></xs:simpleType>
      </xs:element>"#;
    assert!(valid(body, "<r>1 2 3</r>"));
    assert!(valid(body, "<r>  1   2  </r>"), "whitespace separates");
    assert!(!valid(body, "<r>1 two 3</r>"));
}

#[test]
fn a_list_may_name_a_local_item_type() {
    let body = r#"
      <xs:simpleType name="small">
        <xs:restriction base="xs:integer">
          <xs:maxInclusive value="9"/>
        </xs:restriction>
      </xs:simpleType>
      <xs:element name="r">
        <xs:simpleType><xs:list itemType="small"/></xs:simpleType>
      </xs:element>"#;
    assert!(valid(body, "<r>1 2 9</r>"));
    assert!(!valid(body, "<r>1 10</r>"), "the item facet applies");
}

/// A type used many times is parsed once, and a schema that would
/// expand without bound is refused rather than exhausting memory.
///
/// A referenced type is *inlined* where it is used, so a type naming
/// the previous one twice doubles at every level. Memoising the parse
/// does not help: the cost is the materialised tree, not the work to
/// build it. This test was written expecting 24 levels to parse, and
/// instead took the process down — which is how the bound came to
/// exist.
#[test]
fn a_schema_that_would_expand_without_bound_is_refused() {
    let doubling = |levels: usize| {
        let mut body = String::from(
            r#"<xs:complexType name="t0">
                 <xs:sequence><xs:element name="leaf" type="xs:string"/></xs:sequence>
               </xs:complexType>"#,
        );
        for i in 1..=levels {
            let prev = i - 1;
            let _ = write!(
                body,
                r#"<xs:complexType name="t{i}"><xs:sequence>
                     <xs:element name="a" type="t{prev}"/>
                     <xs:element name="b" type="t{prev}"/>
                   </xs:sequence></xs:complexType>"#
            );
        }
        let _ = write!(body, r#"<xs:element name="r" type="t{levels}"/>"#);
        body
    };

    // Ten levels is about a thousand particles: well within the bound.
    assert!(
        parse_schema(&schema(&doubling(10))).is_ok(),
        "a schema of ordinary size must still parse"
    );

    // Twenty-four is sixteen million, from a few kilobytes of input.
    let e = parse_schema(&schema(&doubling(24)))
        .expect_err("must be refused, not attempted");
    assert!(
        e.to_string().contains("particles"),
        "the error should say what the limit was: {e}"
    );
}
