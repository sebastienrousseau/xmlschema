// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 oxml. All rights reserved.

//! Validating a document against a schema, and reading the report.
//!
//! Run with:
//!
//! ```text
//! cargo run --example validate
//! ```

use xmlschema::{parse_schema, validate};

const SCHEMA: &str = r#"<?xml version="1.0"?>
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:element name="order">
    <xs:complexType>
      <xs:sequence>
        <xs:element name="customer" type="xs:string"/>
        <xs:element name="line" maxOccurs="unbounded">
          <xs:complexType>
            <xs:sequence>
              <xs:element name="sku" type="xs:string"/>
              <xs:element name="qty" type="xs:integer"/>
            </xs:sequence>
          </xs:complexType>
        </xs:element>
      </xs:sequence>
      <xs:attribute name="id" type="xs:string" use="required"/>
    </xs:complexType>
  </xs:element>
</xs:schema>"#;

const VALID: &str = r#"<order id="A-1">
  <customer>Ada</customer>
  <line><sku>ABC</sku><qty>2</qty></line>
  <line><sku>DEF</sku><qty>1</qty></line>
</order>"#;

/// Missing the required attribute, an element out of order, and a
/// value that is not an integer.
const INVALID: &str = r"<order>
  <line><qty>many</qty><sku>ABC</sku></line>
  <customer>Ada</customer>
</order>";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // A schema is parsed once and reused. Parsing is the expensive
    // half; validating is the half you repeat.
    let schema = parse_schema(SCHEMA)?;

    println!("== a conforming document ==");
    let doc = oxml::parse(VALID)?;
    let report = validate(&doc, &schema);
    println!("  valid: {}", report.is_valid());
    assert!(report.is_valid(), "the document conforms");

    println!("\n== a document with several problems ==");
    let doc = oxml::parse(INVALID)?;
    let report = validate(&doc, &schema);
    println!("  valid: {}", report.is_valid());
    assert!(!report.is_valid());

    // Every violation is reported, not just the first. A document can
    // be wrong in several independent ways, and fixing them one build
    // at a time is miserable.
    for violation in &report.violations {
        println!("  {}: {}", violation.path, violation.message);
    }
    assert!(
        report.violations.len() > 1,
        "every violation, not only the first"
    );

    println!("\n== a schema that does not parse ==");
    // A malformed schema is a different failure from an invalid
    // document, and has its own error type.
    match parse_schema("<xs:schema") {
        Ok(_) => println!("  unexpectedly parsed"),
        Err(e) => println!("  {e}"),
    }
    Ok(())
}
