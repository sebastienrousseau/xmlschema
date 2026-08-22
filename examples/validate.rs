// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 xmlschema. All rights reserved.

//! Validate a document against a schema and print every violation.
//!
//! ```text
//! cargo run --example validate
//! ```

use xmlschema::{parse_schema, validate};

const SCHEMA: &str = r#"
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:simpleType name="currency">
    <xs:restriction base="xs:string">
      <xs:pattern value="[A-Z]{3}"/>
    </xs:restriction>
  </xs:simpleType>

  <xs:element name="invoice">
    <xs:complexType>
      <xs:sequence>
        <xs:element name="issued" type="xs:date"/>
        <xs:element name="line" minOccurs="1" maxOccurs="unbounded">
          <xs:complexType>
            <xs:sequence>
              <xs:element name="description" type="xs:string"/>
              <xs:element name="amount">
                <xs:simpleType>
                  <xs:restriction base="xs:decimal">
                    <xs:minExclusive value="0"/>
                  </xs:restriction>
                </xs:simpleType>
              </xs:element>
            </xs:sequence>
            <xs:attribute name="currency" type="currency" use="required"/>
          </xs:complexType>
        </xs:element>
      </xs:sequence>
    </xs:complexType>
  </xs:element>
</xs:schema>
"#;

const GOOD: &str = r#"
<invoice>
  <issued>2026-08-22</issued>
  <line currency="GBP">
    <description>Consulting</description>
    <amount>1250.00</amount>
  </line>
</invoice>
"#;

// Four separate problems, each of a different kind.
const BAD: &str = r#"
<invoice>
  <issued>22/08/2026</issued>
  <line currency="pounds">
    <description>Consulting</description>
    <amount>-5</amount>
  </line>
  <line>
    <description>Expenses</description>
    <amount>not a number</amount>
  </line>
</invoice>
"#;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let schema = parse_schema(SCHEMA)?;

    for (label, xml) in [("conforming", GOOD), ("broken", BAD)] {
        println!("== {label} ==");
        let doc = oxml::parse(xml)?;
        let report = validate(&doc, &schema);

        if report.is_valid() {
            println!("  valid\n");
            continue;
        }
        println!("  {} violation(s):", report.violations.len());
        for v in &report.violations {
            println!("    {} — {}", v.path, v.message);
        }
        println!();
    }

    Ok(())
}
