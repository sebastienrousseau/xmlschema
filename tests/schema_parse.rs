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
fn a_schema_with_no_top_level_elements_is_rejected() {
    let e = parse_schema(&schema("")).expect_err("nothing declared");
    assert!(e.to_string().contains("top-level"), "{e}");
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
