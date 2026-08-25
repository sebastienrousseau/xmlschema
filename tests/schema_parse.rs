// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 xmlschema. All rights reserved.

//! Reading `.xsd` documents, including the ways they can be wrong.
//!
//! A schema that fails to parse must say why. Silently producing an
//! empty schema is the worst outcome available: every document then
//! validates, and the caller believes it was checked.

use xmlschema::{Content, parse_schema, validate};

fn schema(body: &str) -> String {
    format!(
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">{body}</xs:schema>"#
    )
}

#[test]
fn a_minimal_schema_parses() {
    let s =
        parse_schema(&schema(r#"<xs:element name="note" type="xs:string"/>"#))
            .expect("valid schema");
    assert!(s.element("note").is_some());
    assert!(s.element("missing").is_none());
}

#[test]
fn a_target_namespace_is_recorded() {
    let s = parse_schema(
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
             targetNamespace="urn:example">
             <xs:element name="a" type="xs:string"/>
           </xs:schema>"#,
    )
    .expect("valid schema");
    assert_eq!(s.target_namespace.as_deref(), Some("urn:example"));
}

#[test]
fn a_schema_without_a_target_namespace_has_none() {
    let s = parse_schema(&schema(r#"<xs:element name="a" type="xs:string"/>"#))
        .expect("valid schema");
    assert_eq!(s.target_namespace, None);
}

#[test]
fn malformed_xml_is_reported_as_such() {
    let e = parse_schema("<xs:schema><unclosed></xs:schema>")
        .expect_err("not well-formed");
    assert!(e.to_string().contains("well-formed"), "{e}");
}

#[test]
fn an_empty_document_has_no_root_element() {
    let e = parse_schema("").expect_err("no root");
    assert!(!e.to_string().is_empty());
}

#[test]
fn a_non_schema_root_is_rejected() {
    // Pointing at the wrong file is a common mistake, and validating
    // everything against an empty schema would hide it.
    let e = parse_schema("<html><body/></html>").expect_err("not a schema");
    assert!(e.to_string().contains("xs:schema"), "{e}");
}

#[test]
fn a_schema_need_not_declare_a_top_level_element() {
    // This used to assert the opposite. A schema whose purpose is to
    // be imported declares types, groups or attributes and no
    // elements at all, and refusing those rejected 282 schemas the
    // W3C suite calls valid.
    let only_a_type = parse_schema(&schema(
        r#"<xs:simpleType name="code">
             <xs:restriction base="xs:string">
               <xs:maxLength value="4"/>
             </xs:restriction>
           </xs:simpleType>"#,
    ))
    .expect("a schema of types alone is valid");
    assert!(only_a_type.elements.is_empty());
    assert!(only_a_type.named_simple_types.contains_key("code"));

    // An entirely empty schema is still a schema.
    assert!(parse_schema(&schema("")).is_ok());
}

/// A reference this schema cannot resolve names something in an
/// imported namespace, which is unenforceable here -- not invalid.
#[test]
fn an_unresolvable_reference_is_not_a_schema_error() {
    let s = parse_schema(&schema(
        r#"<xs:element name="r">
             <xs:complexType><xs:sequence>
               <xs:element ref="other:thing"/>
             </xs:sequence></xs:complexType>
           </xs:element>"#,
    ))
    .expect("an unresolved ref does not invalidate the schema");

    // It keeps its name and cardinality so ordering still works, and
    // accepts any content because there is nothing to check against.
    let doc = oxml::parse("<r><thing>anything at all</thing></r>")
        .expect("well-formed");
    assert!(validate(&doc, &s).is_valid());
}

#[test]
fn an_element_without_a_name_is_rejected() {
    let e = parse_schema(&schema(r#"<xs:element type="xs:string"/>"#))
        .expect_err("no name");
    assert!(e.to_string().contains("name"), "{e}");
}

#[test]
fn cardinality_attributes_are_read() {
    let s = parse_schema(&schema(
        r#"<xs:element name="r">
             <xs:complexType><xs:sequence>
               <xs:element name="a" type="xs:string" minOccurs="0"/>
               <xs:element name="b" type="xs:string" maxOccurs="unbounded"/>
               <xs:element name="c" type="xs:string" minOccurs="2" maxOccurs="5"/>
             </xs:sequence></xs:complexType>
           </xs:element>"#,
    ))
    .expect("valid schema");

    let Some(Content::Sequence(parts)) = s.element("r").map(|p| &*p.content)
    else {
        panic!("expected a sequence");
    };
    assert_eq!(parts[0].occurs.min, 0);
    assert_eq!(parts[0].occurs.max, Some(1));
    assert_eq!(parts[1].occurs.max, None, "unbounded");
    assert_eq!(parts[2].occurs.min, 2);
    assert_eq!(parts[2].occurs.max, Some(5));
}

#[test]
fn an_unparseable_cardinality_falls_back_rather_than_failing() {
    // A malformed maxOccurs should not take the whole schema down; the
    // XSD default is the safest reading.
    let s = parse_schema(&schema(
        r#"<xs:element name="r">
             <xs:complexType><xs:sequence>
               <xs:element name="a" type="xs:string" maxOccurs="lots"/>
               <xs:element name="b" type="xs:string" minOccurs="many"/>
             </xs:sequence></xs:complexType>
           </xs:element>"#,
    ))
    .expect("valid schema");
    let Some(Content::Sequence(parts)) = s.element("r").map(|p| &*p.content)
    else {
        panic!("expected a sequence");
    };
    assert_eq!(parts[0].occurs.max, Some(1));
    assert_eq!(parts[1].occurs.min, 1);
}

#[test]
fn xs_all_permits_any_order_and_forbids_repetition() {
    // This used to assert that `xs:all` was *rejected*, on the
    // reasoning that accepting it and validating nothing would be
    // worse than refusing. That was right while it was unimplemented.
    // It is implemented now, and the property worth pinning is that it
    // is not a sequence with the ordering relaxed.
    let s = parse_schema(&schema(
        r#"<xs:element name="r">
             <xs:complexType><xs:all>
               <xs:element name="a" type="xs:string"/>
               <xs:element name="b" type="xs:string"/>
             </xs:all></xs:complexType>
           </xs:element>"#,
    ))
    .expect("xs:all parses");

    let valid = |xml: &str| {
        let doc = oxml::parse(xml).expect("well-formed");
        validate(&doc, &s).is_valid()
    };
    assert!(valid("<r><a>1</a><b>2</b></r>"), "declared order");
    assert!(valid("<r><b>2</b><a>1</a></r>"), "any order is the point");
    assert!(!valid("<r><a>1</a><a>2</a><b>3</b></r>"), "at most once");
    assert!(!valid("<r><a>1</a></r>"), "b is required");
    assert!(
        !valid("<r><a>1</a><b>2</b><c>3</c></r>"),
        "c is not declared"
    );
}

#[test]
fn an_element_with_no_type_at_all_is_unconstrained() {
    let s = parse_schema(&schema(r#"<xs:element name="anything"/>"#))
        .expect("valid schema");
    assert!(matches!(
        s.element("anything").map(|p| &*p.content),
        Some(Content::Any)
    ));
}

#[test]
fn a_choice_is_parsed_as_a_choice() {
    let s = parse_schema(&schema(
        r#"<xs:element name="r">
             <xs:complexType><xs:choice>
               <xs:element name="a" type="xs:string"/>
               <xs:element name="b" type="xs:string"/>
             </xs:choice></xs:complexType>
           </xs:element>"#,
    ))
    .expect("valid schema");
    assert!(matches!(
        s.element("r").map(|p| &*p.content),
        Some(Content::Choice(_))
    ));
}

#[test]
fn simple_content_extension_resolves_to_its_base() {
    let s = parse_schema(&schema(
        r#"<xs:element name="priced">
             <xs:complexType><xs:simpleContent>
               <xs:extension base="xs:decimal">
                 <xs:attribute name="currency" type="xs:string"/>
               </xs:extension>
             </xs:simpleContent></xs:complexType>
           </xs:element>"#,
    ))
    .expect("valid schema");
    assert!(matches!(
        s.element("priced").map(|p| &*p.content),
        Some(Content::Simple(_))
    ));
}

#[test]
fn a_named_simple_type_is_resolved_by_reference() {
    let s = parse_schema(&schema(
        r#"<xs:simpleType name="Code">
             <xs:restriction base="xs:string">
               <xs:enumeration value="A"/>
               <xs:enumeration value="B"/>
             </xs:restriction>
           </xs:simpleType>
           <xs:element name="code" type="Code"/>"#,
    ))
    .expect("valid schema");
    let Some(Content::Simple(st)) = s.element("code").map(|p| &*p.content)
    else {
        panic!("expected a simple type");
    };
    assert_eq!(st.facets.enumeration, ["A", "B"]);
}

#[test]
fn attributes_and_their_use_are_read() {
    let s = parse_schema(&schema(
        r#"<xs:element name="r">
             <xs:complexType>
               <xs:sequence/>
               <xs:attribute name="req" type="xs:string" use="required"/>
               <xs:attribute name="opt" type="xs:string"/>
             </xs:complexType>
           </xs:element>"#,
    ))
    .expect("valid schema");
    let attrs = &s.element("r").expect("element").attributes;
    assert_eq!(attrs.len(), 2);
    let req = attrs.iter().find(|a| a.name == "req").expect("req");
    let opt = attrs.iter().find(|a| a.name == "opt").expect("opt");
    assert!(req.required);
    assert!(!opt.required);
}

#[test]
fn the_error_type_displays_its_message() {
    let e = parse_schema("<html/>").expect_err("not a schema");
    assert_eq!(e.to_string(), format!("{e}"));
    assert!(!format!("{e}").is_empty());
}

/// A schema is itself an XML document with a content model, and one
/// that breaks it is invalid however sensible its declarations look.
#[test]
fn a_schema_breaking_xsds_own_structure_is_rejected() {
    // Two annotations, where at most one is permitted.
    assert!(
        parse_schema(&schema(
            r#"<xs:complexType name="t">
                 <xs:annotation><xs:documentation>a</xs:documentation></xs:annotation>
                 <xs:annotation><xs:documentation>b</xs:documentation></xs:annotation>
               </xs:complexType>"#,
        ))
        .is_err(),
        "at most one xs:annotation"
    );

    // An annotation that is not first.
    assert!(
        parse_schema(&schema(
            r#"<xs:complexType name="t">
                 <xs:simpleContent><xs:extension base="xs:string"/></xs:simpleContent>
                 <xs:annotation><xs:documentation>a</xs:documentation></xs:annotation>
               </xs:complexType>"#,
        ))
        .is_err(),
        "xs:annotation must come first"
    );

    // Mutually exclusive children.
    assert!(
        parse_schema(&schema(
            r#"<xs:complexType name="t">
                 <xs:sequence/>
                 <xs:choice/>
               </xs:complexType>"#,
        ))
        .is_err(),
        "a complexType has one content model, not two"
    );

    // Both a named type and an inline one.
    assert!(
        parse_schema(&schema(
            r#"<xs:element name="e" type="xs:string">
                 <xs:simpleType><xs:restriction base="xs:string"/></xs:simpleType>
               </xs:element>"#,
        ))
        .is_err(),
        "a type attribute excludes an inline type"
    );

    // Both `name` and `ref`.
    assert!(
        parse_schema(&schema(r#"<xs:element name="e" ref="other"/>"#,))
            .is_err(),
        "name and ref are exclusive"
    );

    // And a well-formed schema still parses.
    assert!(
        parse_schema(&schema(
            r#"<xs:complexType name="t">
                 <xs:annotation><xs:documentation>a</xs:documentation></xs:annotation>
                 <xs:sequence/>
               </xs:complexType>"#,
        ))
        .is_ok(),
        "one annotation, first, with one content model"
    );
}

/// A wildcard's namespace constraint takes four forms.
#[test]
fn a_wildcard_namespace_constraint_is_read_in_every_form() {
    let with = |ns: &str| {
        format!(
            r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
                          targetNamespace="urn:t" xmlns:t="urn:t">
                 <xs:element name="r">
                   <xs:complexType><xs:sequence>
                     <xs:any namespace='{ns}'  processContents="skip"/>
                   </xs:sequence></xs:complexType>
                 </xs:element>
               </xs:schema>"#
        )
    };
    // Each form parses; `##other` and a list are the ones that were
    // never exercised.
    for ns in [
        "##any",
        "##other",
        "urn:a urn:b",
        "##targetNamespace ##local",
    ] {
        assert!(parse_schema(&with(ns)).is_ok(), "namespace={ns}");
    }
    // A wildcard with no `namespace` attribute means `##any`.
    let bare = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
        <xs:element name="r">
          <xs:complexType><xs:sequence><xs:any/></xs:sequence></xs:complexType>
        </xs:element></xs:schema>"#;
    assert!(parse_schema(bare).is_ok());
}

/// `##other` admits anything outside the target namespace, which
/// includes an unqualified element.
#[test]
fn other_excludes_only_the_target_namespace() {
    let xsd = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
                             targetNamespace="urn:t"
                             xmlns:t="urn:t"
                             elementFormDefault="qualified">
        <xs:element name="r">
          <xs:complexType><xs:sequence>
            <xs:any namespace='##other' processContents="skip"/>
          </xs:sequence></xs:complexType>
        </xs:element></xs:schema>"#;
    let s = parse_schema(xsd).expect("schema parses");
    let ok =
        oxml::parse(r#"<r xmlns="urn:t"><foo xmlns="urn:elsewhere"/></r>"#)
            .expect("well-formed");
    assert!(
        validate(&ok, &s).is_valid(),
        "a foreign namespace is admitted"
    );

    let no = oxml::parse(r#"<r xmlns="urn:t"><foo xmlns="urn:t"/></r>"#)
        .expect("well-formed");
    assert!(!validate(&no, &s).is_valid(), "the target namespace is not");
}

/// A document that is not a schema is rejected, and says why.
#[test]
fn a_document_that_is_not_a_schema_is_rejected() {
    // Well-formed XML, but not an xs:schema.
    let e = parse_schema("<notSchema/>").expect_err("not a schema");
    assert!(e.to_string().contains("xs:schema"), "{e}");

    // Not well-formed at all.
    let e = parse_schema("<a><unclosed></a>").expect_err("not well-formed");
    assert!(e.to_string().contains("well-formed"), "{e}");

    // A document with no root element is not well-formed XML, so the
    // parser refuses it before a schema is ever considered — which is
    // why `validate`'s own no-root branch cannot be reached through
    // the public API.
    assert!(oxml::parse("<!-- nothing -->").is_err());
}

/// Two element declarations of the same name in one content model
/// must agree on their type.
///
/// XSD calls this *Element Declarations Consistent*. A model offering
/// `e1` as a string in one branch and as a complex type in another has
/// no single answer for what `e1` is.
#[test]
fn element_declarations_in_one_model_must_agree() {
    let clash = parse_schema(&schema(
        r#"<xs:complexType name="bar">
             <xs:sequence><xs:element name="x" type="xs:string"/></xs:sequence>
           </xs:complexType>
           <xs:element name="doc">
             <xs:complexType><xs:all>
               <xs:element name="e1" type="xs:string"/>
               <xs:element name="e1" type="bar"/>
             </xs:all></xs:complexType>
           </xs:element>"#,
    ));
    assert!(clash.is_err(), "two types for one name");

    // The same type twice is consistent, however often it appears.
    assert!(
        parse_schema(&schema(
            r#"<xs:element name="doc">
                 <xs:complexType><xs:choice>
                   <xs:element name="e1" type="xs:string"/>
                   <xs:element name="e1" type="xs:string"/>
                 </xs:choice></xs:complexType>
               </xs:element>"#,
        ))
        .is_ok(),
        "one type, named twice"
    );

    // The check reaches through nested model groups, because they are
    // the same content model.
    assert!(
        parse_schema(&schema(
            r#"<xs:element name="doc">
                 <xs:complexType><xs:sequence>
                   <xs:element name="e1" type="xs:string"/>
                   <xs:choice>
                     <xs:element name="e1" type="xs:integer"/>
                   </xs:choice>
                 </xs:sequence></xs:complexType>
               </xs:element>"#,
        ))
        .is_err(),
        "a nested group is the same model"
    );

    // It stops at an element's own type, because that is a different
    // content model.
    assert!(
        parse_schema(&schema(
            r#"<xs:element name="doc">
                 <xs:complexType><xs:sequence>
                   <xs:element name="e1" type="xs:string"/>
                   <xs:element name="wrapper">
                     <xs:complexType><xs:sequence>
                       <xs:element name="e1" type="xs:integer"/>
                     </xs:sequence></xs:complexType>
                   </xs:element>
                 </xs:sequence></xs:complexType>
               </xs:element>"#,
        ))
        .is_ok(),
        "a nested type is a different model"
    );
}

/// A complexType with simpleContent or complexContent carries
/// everything inside it.
#[test]
fn attributes_may_not_sit_beside_simple_content() {
    assert!(
        parse_schema(&schema(
            r#"<xs:complexType name="t">
                 <xs:simpleContent><xs:extension base="xs:string"/></xs:simpleContent>
                 <xs:attribute name="a"/>
               </xs:complexType>"#,
        ))
        .is_err(),
        "the attribute belongs inside the extension"
    );

    // Inside, it is fine.
    assert!(
        parse_schema(&schema(
            r#"<xs:complexType name="t">
                 <xs:simpleContent>
                   <xs:extension base="xs:string">
                     <xs:attribute name="a"/>
                   </xs:extension>
                 </xs:simpleContent>
               </xs:complexType>"#,
        ))
        .is_ok()
    );

    // And an annotation may still sit beside it.
    assert!(
        parse_schema(&schema(
            r#"<xs:complexType name="t">
                 <xs:annotation><xs:documentation>a</xs:documentation></xs:annotation>
                 <xs:simpleContent><xs:extension base="xs:string"/></xs:simpleContent>
               </xs:complexType>"#,
        ))
        .is_ok()
    );
}

/// A facet's value must belong to the type it narrows.
#[test]
fn a_facet_value_must_be_valid_for_its_base() {
    let with = |base: &str, facet: &str, value: &str| {
        parse_schema(&schema(&format!(
            r#"<xs:simpleType name="t">
                 <xs:restriction base="{base}">
                   <xs:{facet} value="{value}"/>
                 </xs:restriction>
               </xs:simpleType>"#
        )))
    };
    // `CA` is not an integer, so it cannot be one of an integer's
    // permitted values.
    assert!(with("xs:integer", "enumeration", "CA").is_err());
    assert!(with("xs:integer", "enumeration", "10").is_ok());
    // Bounds too.
    assert!(with("xs:integer", "minInclusive", "x").is_err());
    assert!(with("xs:date", "maxInclusive", "not-a-date").is_err());
    assert!(with("xs:date", "maxInclusive", "2001-01-01").is_ok());
    // A count is a count whatever the base is.
    assert!(with("xs:string", "maxLength", "-1").is_err());
    assert!(with("xs:string", "maxLength", "four").is_err());
    assert!(with("xs:string", "maxLength", "4").is_ok());
    // A pattern is not a value of the base type, so it is not checked
    // against it.
    assert!(with("xs:integer", "pattern", "[0-9]+").is_ok());
}

/// A type may not declare two attributes of the same name.
#[test]
fn two_attributes_of_one_name_are_rejected() {
    // Declared twice outright.
    assert!(
        parse_schema(&schema(
            r#"<xs:complexType name="t">
                 <xs:attribute name="a" type="xs:string"/>
                 <xs:attribute name="a" type="xs:integer"/>
               </xs:complexType>"#,
        ))
        .is_err()
    );

    // Declared once and referenced once is still twice: `ref="foo"`
    // and `name="foo"` name the same attribute.
    assert!(
        parse_schema(&schema(
            r#"<xs:attribute name="foo" type="xs:string"/>
               <xs:attributeGroup name="g">
                 <xs:attribute name="foo" type="xs:int"/>
                 <xs:attribute ref="foo"/>
               </xs:attributeGroup>"#,
        ))
        .is_err()
    );

    // Two different names are fine.
    assert!(
        parse_schema(&schema(
            r#"<xs:complexType name="t">
                 <xs:attribute name="a" type="xs:string"/>
                 <xs:attribute name="b" type="xs:string"/>
               </xs:complexType>"#,
        ))
        .is_ok()
    );
}
