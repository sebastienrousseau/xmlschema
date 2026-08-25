// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 xmlschema. All rights reserved.

//! Restriction facets.
//!
//! A facet that is parsed but never enforced is worse than one that is
//! missing: the schema says the constraint exists and nothing applies
//! it. Each is checked against a value it must accept and one it must
//! reject, and the rejection message is checked for the bound itself,
//! since that is what tells an author what to change.

use xmlschema::{parse_schema, validate};

/// Build a schema whose `v` element restricts `base` with `facets`.
fn restricted(base: &str, facets: &str) -> xmlschema::Schema {
    let xsd = format!(
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
             <xs:element name="v">
               <xs:simpleType>
                 <xs:restriction base="{base}">{facets}</xs:restriction>
               </xs:simpleType>
             </xs:element>
           </xs:schema>"#
    );
    parse_schema(&xsd).expect("valid schema")
}

fn report(schema: &xmlschema::Schema, value: &str) -> xmlschema::Report {
    let doc = oxml::parse(&format!("<v>{value}</v>")).expect("well-formed");
    validate(&doc, schema)
}

fn accepts(schema: &xmlschema::Schema, value: &str) -> bool {
    report(schema, value).violations.is_empty()
}

#[test]
fn enumeration_admits_only_listed_values() {
    let s = restricted(
        "xs:string",
        r#"<xs:enumeration value="red"/><xs:enumeration value="green"/>"#,
    );
    assert!(accepts(&s, "red"));
    assert!(accepts(&s, "green"));
    assert!(!accepts(&s, "blue"));
    assert!(!accepts(&s, ""));
    assert!(!accepts(&s, "RED"), "enumeration is case sensitive");
}

#[test]
fn an_enumeration_violation_lists_the_permitted_values() {
    let s = restricted(
        "xs:string",
        r#"<xs:enumeration value="red"/><xs:enumeration value="green"/>"#,
    );
    let text = report(&s, "blue").to_string();
    assert!(text.contains("red"), "{text}");
    assert!(text.contains("green"), "{text}");
}

#[test]
fn length_requires_exactly_that_many_characters() {
    let s = restricted("xs:string", r#"<xs:length value="3"/>"#);
    assert!(accepts(&s, "abc"));
    assert!(!accepts(&s, "ab"));
    assert!(!accepts(&s, "abcd"));
    let text = report(&s, "ab").to_string();
    assert!(text.contains('3'), "{text}");
}

#[test]
fn min_and_max_length_bound_each_end() {
    let s = restricted(
        "xs:string",
        r#"<xs:minLength value="2"/><xs:maxLength value="4"/>"#,
    );
    assert!(!accepts(&s, "a"));
    assert!(accepts(&s, "ab"));
    assert!(accepts(&s, "abcd"));
    assert!(!accepts(&s, "abcde"));

    assert!(report(&s, "a").to_string().contains('2'));
    assert!(report(&s, "abcde").to_string().contains('4'));
}

#[test]
fn length_counts_characters_not_bytes() {
    // A multi-byte character is one character; counting bytes would
    // reject valid values in every non-ASCII document.
    let s = restricted("xs:string", r#"<xs:length value="3"/>"#);
    assert!(
        accepts(&s, "é中x"),
        "three characters, more than three bytes"
    );
    assert!(!accepts(&s, "é中"));
}

#[test]
fn inclusive_bounds_include_their_endpoints() {
    let s = restricted(
        "xs:integer",
        r#"<xs:minInclusive value="1"/><xs:maxInclusive value="10"/>"#,
    );
    assert!(!accepts(&s, "0"));
    assert!(accepts(&s, "1"), "minInclusive must admit its endpoint");
    assert!(accepts(&s, "10"), "maxInclusive must admit its endpoint");
    assert!(!accepts(&s, "11"));
}

#[test]
fn exclusive_bounds_exclude_their_endpoints() {
    let s = restricted(
        "xs:integer",
        r#"<xs:minExclusive value="1"/><xs:maxExclusive value="10"/>"#,
    );
    assert!(!accepts(&s, "1"), "minExclusive must reject its endpoint");
    assert!(accepts(&s, "2"));
    assert!(accepts(&s, "9"));
    assert!(!accepts(&s, "10"), "maxExclusive must reject its endpoint");
}

#[test]
fn a_bound_violation_names_the_bound() {
    let s = restricted("xs:integer", r#"<xs:maxInclusive value="10"/>"#);
    let text = report(&s, "11").to_string();
    assert!(text.contains("10"), "{text}");
}

#[test]
fn a_pattern_facet_is_enforced() {
    let s =
        restricted("xs:string", r#"<xs:pattern value="[A-Z]{2}[0-9]{3}"/>"#);
    assert!(accepts(&s, "AB123"));
    assert!(!accepts(&s, "ab123"));
    assert!(!accepts(&s, "AB12"));
    assert!(!accepts(&s, "XAB123"), "patterns are anchored");
}

#[test]
fn facets_compose_and_every_failure_is_reported() {
    let s = restricted(
        "xs:string",
        r#"<xs:minLength value="3"/><xs:pattern value="[a-z]+"/>"#,
    );
    assert!(accepts(&s, "abc"));
    assert!(!accepts(&s, "ab"), "too short");
    assert!(!accepts(&s, "ABC"), "wrong pattern");
}

#[test]
fn an_attribute_is_validated_against_its_type() {
    let xsd = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
        <xs:element name="r">
          <xs:complexType>
            <xs:sequence/>
            <xs:attribute name="n" type="xs:integer" use="required"/>
          </xs:complexType>
        </xs:element>
      </xs:schema>"#;
    let s = parse_schema(xsd).expect("valid schema");

    let ok = oxml::parse(r#"<r n="42"/>"#).expect("well-formed");
    assert!(validate(&ok, &s).violations.is_empty());

    let bad = oxml::parse(r#"<r n="forty"/>"#).expect("well-formed");
    assert!(!validate(&bad, &s).violations.is_empty());

    let missing = oxml::parse("<r/>").expect("well-formed");
    let report = validate(&missing, &s);
    assert!(!report.violations.is_empty(), "required attribute absent");
    assert!(report.to_string().contains('n'), "{report}");
}

#[test]
fn an_attribute_with_an_inline_simple_type_is_constrained() {
    let xsd = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
        <xs:element name="r">
          <xs:complexType>
            <xs:sequence/>
            <xs:attribute name="code">
              <xs:simpleType>
                <xs:restriction base="xs:string">
                  <xs:enumeration value="A"/>
                </xs:restriction>
              </xs:simpleType>
            </xs:attribute>
          </xs:complexType>
        </xs:element>
      </xs:schema>"#;
    let s = parse_schema(xsd).expect("valid schema");

    let ok = oxml::parse(r#"<r code="A"/>"#).expect("well-formed");
    assert!(validate(&ok, &s).violations.is_empty());

    let bad = oxml::parse(r#"<r code="B"/>"#).expect("well-formed");
    assert!(!validate(&bad, &s).violations.is_empty());
}

#[test]
fn an_attribute_with_no_declared_type_accepts_anything() {
    let xsd = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
        <xs:element name="r">
          <xs:complexType>
            <xs:sequence/>
            <xs:attribute name="free"/>
          </xs:complexType>
        </xs:element>
      </xs:schema>"#;
    let s = parse_schema(xsd).expect("valid schema");
    let doc = oxml::parse(r#"<r free="anything at all"/>"#).expect("ok");
    assert!(validate(&doc, &s).violations.is_empty());
}

#[test]
fn an_unrecognised_base_type_falls_back_to_string() {
    // An unknown base must not silently reject every value.
    let s = restricted("xs:madeUpType", r#"<xs:minLength value="2"/>"#);
    assert!(accepts(&s, "ab"));
    assert!(!accepts(&s, "a"));
}

/// `xs:totalDigits` and `xs:fractionDigits` count *significant*
/// digits, which is a property of the value and not of how it was
/// written.
#[test]
fn digit_facets_count_significant_digits() {
    let xsd = r#"
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:element name="v">
    <xs:simpleType>
      <xs:restriction base="xs:decimal">
        <xs:totalDigits value="3"/>
        <xs:fractionDigits value="1"/>
      </xs:restriction>
    </xs:simpleType>
  </xs:element>
</xs:schema>"#;
    let schema = parse_schema(xsd).expect("schema parses");
    let accepts = |v: &str| {
        let doc = oxml::parse(&format!("<v>{v}</v>")).expect("well-formed");
        validate(&doc, &schema).is_valid()
    };
    assert!(accepts("12.3"), "three total, one fraction");
    assert!(
        accepts("1.0"),
        "trailing fraction zeros are not significant"
    );
    assert!(accepts("012.3"), "leading zeros are not significant");
    assert!(accepts("0"), "zero has one significant digit");
    assert!(accepts("-12.3"), "the sign is not a digit");
    assert!(!accepts("1234"), "four total digits");
    assert!(!accepts("1.23"), "two fraction digits");
}

/// `xs:whiteSpace` decides what the value *is*, so it applies before
/// every other check.
#[test]
fn the_whitespace_facet_applies_before_validation() {
    let with = |rule: &str| {
        format!(
            r#"
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:element name="v">
    <xs:simpleType>
      <xs:restriction base="xs:string">
        <xs:whiteSpace value="{rule}"/>
        <xs:maxLength value="3"/>
      </xs:restriction>
    </xs:simpleType>
  </xs:element>
</xs:schema>"#
        )
    };
    let accepts = |rule: &str, v: &str| {
        let schema = parse_schema(&with(rule)).expect("schema parses");
        let doc = oxml::parse(&format!("<v>{v}</v>")).expect("well-formed");
        validate(&doc, &schema).is_valid()
    };
    // "  a  " is five characters preserved, one collapsed.
    assert!(!accepts("preserve", "  a  "), "five characters");
    assert!(accepts("collapse", "  a  "), "collapses to one");
    // Replace turns tabs into spaces without collapsing them.
    assert!(!accepts("replace", "\ta\tb\t"), "still five characters");
    assert!(accepts("collapse", "\ta\tb\t"), "collapses to three");
}
