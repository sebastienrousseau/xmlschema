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

/// `xs:any` matches by namespace, and `processContents` decides how
/// hard the match is validated.
#[test]
fn a_wildcard_matches_by_namespace() {
    let xsd = r#"
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:element name="known" type="xs:integer"/>
  <xs:element name="r">
    <xs:complexType><xs:sequence>
      <xs:element name="a" type="xs:string"/>
      <xs:any namespace='##any' processContents="lax" maxOccurs="unbounded"/>
    </xs:sequence></xs:complexType>
  </xs:element>
</xs:schema>"#;
    let schema = parse_schema(xsd).expect("schema parses");
    let valid = |xml: &str| {
        let doc = oxml::parse(xml).expect("well-formed");
        validate(&doc, &schema).is_valid()
    };
    // The wildcard admits an element the type never declared.
    assert!(valid("<r><a>x</a><anything/></r>"));
    assert!(valid("<r><a>x</a><one/><two/></r>"));
    // Under `lax` a matched element *with* a declaration is still
    // validated against it.
    assert!(valid("<r><a>x</a><known>42</known></r>"));
    assert!(
        !valid("<r><a>x</a><known>not a number</known></r>"),
        "lax still validates what it can resolve"
    );
    // The wildcard does not excuse the declared particle.
    assert!(!valid("<r><anything/></r>"), "`a` is still required");
}

/// A strict wildcard requires the matched element to be declared.
#[test]
fn a_strict_wildcard_requires_a_declaration() {
    let xsd = r#"
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:element name="known" type="xs:string"/>
  <xs:element name="r">
    <xs:complexType><xs:sequence>
      <xs:any namespace='##any' processContents="strict"/>
    </xs:sequence></xs:complexType>
  </xs:element>
</xs:schema>"#;
    let schema = parse_schema(xsd).expect("schema parses");
    let valid = |xml: &str| {
        let doc = oxml::parse(xml).expect("well-formed");
        validate(&doc, &schema).is_valid()
    };
    assert!(valid("<r><known>x</known></r>"));
    assert!(!valid("<r><undeclared/></r>"), "strict needs a declaration");
}

/// `xs:anyAttribute` admits attributes the type does not declare.
#[test]
fn an_attribute_wildcard_admits_undeclared_attributes() {
    let xsd = r#"
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:element name="r">
    <xs:complexType>
      <xs:attribute name="known" type="xs:integer"/>
      <xs:anyAttribute namespace='##any' processContents="lax"/>
    </xs:complexType>
  </xs:element>
</xs:schema>"#;
    let schema = parse_schema(xsd).expect("schema parses");
    let valid = |xml: &str| {
        let doc = oxml::parse(xml).expect("well-formed");
        validate(&doc, &schema).is_valid()
    };
    assert!(valid(r#"<r other="anything"/>"#));
    // The declared one is still checked.
    assert!(!valid(r#"<r known="not a number"/>"#));
}

/// `xs:all` reports each way a document can break it.
#[test]
fn the_all_group_reports_what_went_wrong() {
    let xsd = r#"
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:element name="r">
    <xs:complexType><xs:all>
      <xs:element name="a" type="xs:string"/>
      <xs:element name="b" type="xs:integer" minOccurs="0"/>
    </xs:all></xs:complexType>
  </xs:element>
</xs:schema>"#;
    let schema = parse_schema(xsd).expect("schema parses");
    let report = |xml: &str| {
        let doc = oxml::parse(xml).expect("well-formed");
        validate(&doc, &schema)
    };
    assert!(report("<r><a>x</a></r>").is_valid(), "b is optional");
    assert!(report("<r><b>1</b><a>x</a></r>").is_valid(), "any order");

    // A repeat, an undeclared child, a missing required child, and a
    // child whose own type is violated: each is reported.
    for (xml, expect) in [
        ("<r><a>x</a><a>y</a></r>", "more than once"),
        ("<r><a>x</a><c/></r>", "not permitted"),
        ("<r><b>1</b></r>", "missing required"),
        ("<r><a>x</a><b>no</b></r>", "integer"),
    ] {
        let r = report(xml);
        assert!(!r.is_valid(), "{xml} should be invalid");
        assert!(
            r.violations.iter().any(|v| v.message.contains(expect)),
            "{xml}: expected a message about `{expect}`, got {:?}",
            r.violations
        );
    }
}

/// A union reports that nothing matched, and a list says which item.
#[test]
fn simple_type_varieties_report_usefully() {
    let xsd = r#"
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:element name="u">
    <xs:simpleType><xs:union memberTypes="xs:integer xs:date"/></xs:simpleType>
  </xs:element>
  <xs:element name="l">
    <xs:simpleType>
      <xs:restriction>
        <xs:simpleType><xs:list itemType="xs:integer"/></xs:simpleType>
        <xs:length value="2"/>
      </xs:restriction>
    </xs:simpleType>
  </xs:element>
</xs:schema>"#;
    let schema = parse_schema(xsd).expect("schema parses");
    let report = |xml: &str| {
        let doc = oxml::parse(xml).expect("well-formed");
        validate(&doc, &schema)
    };

    let r = report("<u>neither</u>");
    assert!(!r.is_valid());
    assert!(
        r.violations[0].message.contains("member types"),
        "{:?}",
        r.violations
    );

    let r = report("<l>1 two</l>");
    assert!(!r.is_valid());
    assert!(
        r.violations[0].message.contains("list item"),
        "{:?}",
        r.violations
    );

    // Length counts items, and says so.
    let r = report("<l>1 2 3</l>");
    assert!(!r.is_valid());
    assert!(
        r.violations[0].message.contains("items"),
        "{:?}",
        r.violations
    );
    assert!(report("<l>1 2</l>").is_valid());
}

/// A document whose root has no declaration cannot be validated.
#[test]
fn an_undeclared_root_is_reported_rather_than_ignored() {
    let xsd = r#"
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:element name="known" type="xs:string"/>
</xs:schema>"#;
    let schema = parse_schema(xsd).expect("schema parses");
    let doc = oxml::parse("<unknown/>").expect("well-formed");
    let report = validate(&doc, &schema);
    assert!(!report.is_valid());
    assert!(!report.violations.is_empty());
}

/// A violation carries where it happened, not only what.
#[test]
fn a_violation_carries_its_path() {
    let xsd = r#"
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:element name="r">
    <xs:complexType><xs:sequence>
      <xs:element name="n" type="xs:integer" maxOccurs="unbounded"/>
    </xs:sequence></xs:complexType>
  </xs:element>
</xs:schema>"#;
    let schema = parse_schema(xsd).expect("schema parses");
    let doc = oxml::parse("<r><n>1</n><n>bad</n></r>").expect("well-formed");
    let report = validate(&doc, &schema);
    assert!(!report.is_valid());
    // The second child, identified positionally.
    assert!(
        report.violations[0].path.contains("[2]"),
        "path should locate the second `n`: {:?}",
        report.violations
    );
    // And the report renders.
    assert!(!report.to_string().is_empty());
}

/// A report renders every violation it holds.
#[test]
fn a_report_renders_its_violations() {
    let xsd = r#"
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:element name="r">
    <xs:complexType><xs:sequence>
      <xs:element name="a" type="xs:integer"/>
      <xs:element name="b" type="xs:integer"/>
    </xs:sequence></xs:complexType>
  </xs:element>
</xs:schema>"#;
    let schema = parse_schema(xsd).expect("schema parses");
    let doc = oxml::parse("<r><a>x</a><b>y</b></r>").expect("well-formed");
    let report = validate(&doc, &schema);
    assert!(!report.is_valid());
    let text = report.to_string();
    // Both violations appear, each with its path.
    assert!(text.contains("/r/a"), "{text}");
    assert!(text.contains("/r/b"), "{text}");

    // And a clean report says so in one word.
    let ok = oxml::parse("<r><a>1</a><b>2</b></r>").expect("well-formed");
    assert_eq!(validate(&ok, &schema).to_string(), "valid");
}

/// A fixed *attribute* value must be exactly that value.
#[test]
fn a_fixed_attribute_value_is_enforced() {
    let xsd = r#"
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:element name="r">
    <xs:complexType>
      <xs:attribute name="k" type="xs:string" fixed="only"/>
    </xs:complexType>
  </xs:element>
</xs:schema>"#;
    let schema = parse_schema(xsd).expect("schema parses");
    let valid = |xml: &str| {
        let doc = oxml::parse(xml).expect("well-formed");
        validate(&doc, &schema).is_valid()
    };
    assert!(valid("<r/>"), "fixed does not make it required");
    assert!(valid(r#"<r k="only"/>"#));
    assert!(!valid(r#"<r k="other"/>"#));
}

/// A strict attribute wildcard requires a top-level declaration, and
/// never complains about the schema-instance attributes.
#[test]
fn a_strict_attribute_wildcard_requires_a_declaration() {
    let xsd = r#"
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:attribute name="known" type="xs:string"/>
  <xs:element name="r">
    <xs:complexType>
      <xs:attribute name="own" type="xs:string"/>
      <xs:anyAttribute namespace='##any' processContents="strict"/>
    </xs:complexType>
  </xs:element>
</xs:schema>"#;
    let schema = parse_schema(xsd).expect("schema parses");
    let valid = |xml: &str| {
        let doc = oxml::parse(xml).expect("well-formed");
        validate(&doc, &schema).is_valid()
    };
    assert!(valid(r#"<r own="x"/>"#), "declared on the type");
    assert!(!valid(r#"<r other="x"/>"#), "no declaration anywhere");
    // `xsi:` attributes belong to the schema-instance namespace and
    // are not this schema's to declare.
    assert!(
        valid(
            r#"<r xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:nil="false"/>"#
        ),
        "xsi attributes are defined by the specification"
    );
}

/// A wildcard may name the namespaces it admits explicitly.
#[test]
fn a_wildcard_may_list_its_namespaces() {
    let xsd = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
                              targetNamespace="urn:t" xmlns:t="urn:t"
                              elementFormDefault="qualified">
        <xs:element name="r">
          <xs:complexType><xs:sequence>
            <xs:any namespace="urn:a urn:b" processContents="skip"/>
          </xs:sequence></xs:complexType>
        </xs:element></xs:schema>"#;
    let schema = parse_schema(xsd).expect("schema parses");
    let valid = |xml: &str| {
        let doc = oxml::parse(xml).expect("well-formed");
        validate(&doc, &schema).is_valid()
    };
    assert!(valid(r#"<r xmlns="urn:t"><x xmlns="urn:a"/></r>"#));
    assert!(valid(r#"<r xmlns="urn:t"><x xmlns="urn:b"/></r>"#));
    assert!(
        !valid(r#"<r xmlns="urn:t"><x xmlns="urn:c"/></r>"#),
        "urn:c is not on the list"
    );
}

/// An enumeration on a list constrains the whole value.
#[test]
fn an_enumeration_on_a_list_matches_the_whole_value() {
    let xsd = r#"
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:element name="r">
    <xs:simpleType>
      <xs:restriction>
        <xs:simpleType><xs:list itemType="xs:integer"/></xs:simpleType>
        <xs:enumeration value="1 2"/>
        <xs:enumeration value="3 4"/>
      </xs:restriction>
    </xs:simpleType>
  </xs:element>
</xs:schema>"#;
    let schema = parse_schema(xsd).expect("schema parses");
    let valid = |v: &str| {
        let doc = oxml::parse(&format!("<r>{v}</r>")).expect("well-formed");
        validate(&doc, &schema).is_valid()
    };
    assert!(valid("1 2"));
    assert!(valid("3 4"));
    assert!(!valid("1 3"), "not one of the permitted lists");
    assert!(!valid("1"), "nor is a prefix of one");
}
