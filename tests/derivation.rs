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

/// `maxOccurs` on a model group repeats the group, not its content.
#[test]
fn a_model_group_carries_its_own_cardinality() {
    let body = r#"
      <xs:element name="r">
        <xs:complexType>
          <xs:sequence maxOccurs="unbounded">
            <xs:element name="v" type="xs:integer"/>
          </xs:sequence>
        </xs:complexType>
      </xs:element>"#;
    assert!(valid(body, "<r><v>1</v></r>"));
    assert!(
        valid(body, "<r><v>1</v><v>2</v><v>3</v></r>"),
        "the group repeats"
    );
    assert!(!valid(body, "<r><v>x</v></r>"), "the type still applies");

    // A bounded repeat multiplies through.
    let twice = r#"
      <xs:element name="r">
        <xs:complexType>
          <xs:sequence maxOccurs="2">
            <xs:element name="v" type="xs:string"/>
          </xs:sequence>
        </xs:complexType>
      </xs:element>"#;
    assert!(valid(twice, "<r><v>a</v><v>b</v></r>"));
    assert!(
        !valid(twice, "<r><v>a</v><v>b</v><v>c</v></r>"),
        "at most two"
    );

    // An optional group makes its single particle optional.
    let optional = r#"
      <xs:element name="r">
        <xs:complexType>
          <xs:sequence minOccurs="0">
            <xs:element name="v" type="xs:string"/>
          </xs:sequence>
        </xs:complexType>
      </xs:element>"#;
    assert!(valid(optional, "<r/>"));
    assert!(valid(optional, "<r><v>a</v></r>"));
}

/// A repeated group of *more* than one particle is not modelled, and
/// says so rather than guessing.
#[test]
fn a_repeated_multi_particle_group_is_reported_as_unenforceable() {
    let xsd = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
             <xs:element name="r">
               <xs:complexType>
                 <xs:sequence maxOccurs="2">
                   <xs:element name="a" type="xs:string"/>
                   <xs:element name="b" type="xs:string"/>
                 </xs:sequence>
               </xs:complexType>
             </xs:element>
           </xs:schema>"#;
    let doc = oxml::parse(xsd).expect("well-formed");
    let gaps = xmlschema::support::unsupported(&doc);
    assert!(
        gaps.iter().any(|g| g.construct.contains("repeated")),
        "`(a, b){{2}}` permits `a b a b` and not `a a b b`, which this \
         crate does not model: {gaps:?}"
    );
}

/// A derivation restates or appends *one* content model.
#[test]
fn a_derivation_may_hold_one_model_group() {
    let two = parse_schema(&schema(
        r#"<xs:group name="g"><xs:sequence>
             <xs:element name="a" type="xs:string"/>
           </xs:sequence></xs:group>
           <xs:complexType name="base">
             <xs:sequence><xs:any/></xs:sequence>
           </xs:complexType>
           <xs:complexType name="derived">
             <xs:complexContent>
               <xs:restriction base="base">
                 <xs:group ref="g"/>
                 <xs:all/>
               </xs:restriction>
             </xs:complexContent>
           </xs:complexType>"#,
    ));
    assert!(two.is_err(), "two models is not a narrower model");

    // One is fine.
    assert!(
        parse_schema(&schema(
            r#"<xs:complexType name="base">
                 <xs:sequence><xs:any/></xs:sequence>
               </xs:complexType>
               <xs:complexType name="derived">
                 <xs:complexContent>
                   <xs:restriction base="base">
                     <xs:sequence><xs:any/></xs:sequence>
                   </xs:restriction>
                 </xs:complexContent>
               </xs:complexType>"#,
        ))
        .is_ok()
    );
}

/// A restricted wildcard may not admit a namespace the base excludes.
///
/// It may, however, validate less strictly: the first edition required
/// `processContents` to be at least as strong and the second-edition
/// errata removed that clause. Enforcing the original rule rejected
/// seven schemas the suite calls valid.
#[test]
fn a_restricted_wildcard_may_not_widen_its_namespaces() {
    let with = |base_ns: &str, derived: &str| {
        parse_schema(&schema(&format!(
            r#"<xs:complexType name="base">
                 <xs:sequence><xs:any namespace="{base_ns}"/></xs:sequence>
               </xs:complexType>
               <xs:complexType name="derived">
                 <xs:complexContent>
                   <xs:restriction base="base">
                     <xs:sequence><xs:any {derived}/></xs:sequence>
                   </xs:restriction>
                 </xs:complexContent>
               </xs:complexType>"#
        )))
    };

    // Narrowing a list is a restriction.
    assert!(with("urn:a urn:b", r#"namespace="urn:a""#).is_ok());
    // Widening past it is not.
    assert!(with("urn:a", r#"namespace="urn:a urn:b""#).is_err());
    assert!(with("urn:a", "namespace='##any'").is_err());
    // Anything is within `##any`.
    assert!(with("##any", r#"namespace="urn:a""#).is_ok());

    // `processContents` may be weakened — the errata removed that
    // constraint, and the suite depends on it.
    assert!(with("##any", r#"processContents="lax""#).is_ok());
    assert!(with("##any", r#"processContents="skip""#).is_ok());
}

/// The subsumption relation, reached through `parse_schema`.
///
/// The relation itself is tested directly in `tests/subsumption.rs`;
/// these check that a schema's content models actually reach it, with
/// their groups resolved and their base types looked up.
#[test]
fn a_restriction_is_checked_against_its_base() {
    let derived_from = |base: &str, restriction: &str| {
        parse_schema(&schema(&format!(
            r#"<xs:complexType name="base">{base}</xs:complexType>
               <xs:complexType name="derived">
                 <xs:complexContent>
                   <xs:restriction base="base">{restriction}</xs:restriction>
                 </xs:complexContent>
               </xs:complexType>"#
        )))
    };

    // Narrowing an occurrence range is a restriction.
    assert!(
        derived_from(
            r#"<xs:sequence><xs:element name="a" maxOccurs="unbounded"/></xs:sequence>"#,
            r#"<xs:sequence><xs:element name="a" maxOccurs="3"/></xs:sequence>"#,
        )
        .is_ok()
    );
    // Widening it is not.
    assert!(
        derived_from(
            r#"<xs:sequence><xs:element name="a" maxOccurs="3"/></xs:sequence>"#,
            r#"<xs:sequence><xs:element name="a" maxOccurs="unbounded"/></xs:sequence>"#,
        )
        .is_err()
    );
    // Renaming an element is not a restriction of it.
    assert!(
        derived_from(
            r#"<xs:sequence><xs:element name="a"/></xs:sequence>"#,
            r#"<xs:sequence><xs:element name="b"/></xs:sequence>"#,
        )
        .is_err()
    );
    // Dropping an optional particle is fine; dropping a required one
    // is not.
    assert!(
        derived_from(
            r#"<xs:sequence><xs:element name="a"/><xs:element name="b" minOccurs="0"/></xs:sequence>"#,
            r#"<xs:sequence><xs:element name="a"/></xs:sequence>"#,
        )
        .is_ok()
    );
    assert!(
        derived_from(
            r#"<xs:sequence><xs:element name="a"/><xs:element name="b"/></xs:sequence>"#,
            r#"<xs:sequence><xs:element name="a"/></xs:sequence>"#,
        )
        .is_err()
    );
    // Three elements restricting three wildcards, decided on the
    // group's total range.
    assert!(
        derived_from(
            r#"<xs:sequence><xs:any minOccurs="3" maxOccurs="3"/></xs:sequence>"#,
            r#"<xs:all><xs:element name="e1"/><xs:element name="e2"/><xs:element name="e3"/></xs:all>"#,
        )
        .is_ok()
    );
}

/// A group reference is resolved before the relation sees it.
#[test]
fn a_restriction_may_state_its_model_through_a_group() {
    let s = parse_schema(&schema(
        r#"<xs:group name="g">
             <xs:sequence><xs:element name="a"/></xs:sequence>
           </xs:group>
           <xs:complexType name="base">
             <xs:sequence>
               <xs:element name="a"/>
               <xs:element name="b" minOccurs="0"/>
             </xs:sequence>
           </xs:complexType>
           <xs:complexType name="derived">
             <xs:complexContent>
               <xs:restriction base="base"><xs:group ref="g"/></xs:restriction>
             </xs:complexContent>
           </xs:complexType>"#,
    ));
    assert!(s.is_ok(), "the group resolves to a valid restriction");
}

/// A derivation chain is walked when comparing element types.
#[test]
fn a_derived_type_may_narrow_an_element_type() {
    let s = parse_schema(&schema(
        r#"<xs:complexType name="parent">
             <xs:sequence><xs:element name="x"/></xs:sequence>
           </xs:complexType>
           <xs:complexType name="child">
             <xs:complexContent>
               <xs:restriction base="parent">
                 <xs:sequence><xs:element name="x"/></xs:sequence>
               </xs:restriction>
             </xs:complexContent>
           </xs:complexType>
           <xs:complexType name="base">
             <xs:sequence><xs:element name="e" type="parent"/></xs:sequence>
           </xs:complexType>
           <xs:complexType name="derived">
             <xs:complexContent>
               <xs:restriction base="base">
                 <xs:sequence><xs:element name="e" type="child"/></xs:sequence>
               </xs:restriction>
             </xs:complexContent>
           </xs:complexType>"#,
    ));
    assert!(s.is_ok(), "`child` derives from `parent`");
}

/// Substitution groups make an element particle stand for its whole
/// group, which this crate does not model — so it declines to decide
/// rather than reporting a name mismatch.
#[test]
fn substitution_groups_suspend_the_check() {
    let s = parse_schema(&schema(
        r#"<xs:element name="head"/>
           <xs:element name="m1" substitutionGroup="head"/>
           <xs:complexType name="base">
             <xs:sequence><xs:element ref="head"/></xs:sequence>
           </xs:complexType>
           <xs:complexType name="derived">
             <xs:complexContent>
               <xs:restriction base="base">
                 <xs:sequence><xs:element ref="m1"/></xs:sequence>
               </xs:restriction>
             </xs:complexContent>
           </xs:complexType>"#,
    ));
    assert!(s.is_ok(), "a member of the group is a valid restriction");
}
