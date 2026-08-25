// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 xmlschema. All rights reserved.

//! Validating a document against an already-parsed schema.
//!
//! This is the repeated cost: a server parses its schema once and
//! validates every request against it. The shapes chosen are the ones
//! whose validators differ — a sequence walks positionally, a choice
//! searches its branches, and `xs:all` counts occurrences.

#![allow(missing_docs)] // criterion_group! generates an undocumented item

use core::fmt::Write as _;
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

const SEQUENCE: &str = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:element name="r"><xs:complexType><xs:sequence>
    <xs:element name="item" maxOccurs="unbounded">
      <xs:complexType>
        <xs:sequence><xs:element name="name" type="xs:string"/></xs:sequence>
        <xs:attribute name="id" type="xs:integer" use="required"/>
      </xs:complexType>
    </xs:element>
  </xs:sequence></xs:complexType></xs:element>
</xs:schema>"#;

const CHOICE: &str = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:element name="r"><xs:complexType><xs:choice maxOccurs="unbounded">
    <xs:element name="a" type="xs:string"/>
    <xs:element name="b" type="xs:integer"/>
    <xs:element name="c" type="xs:date"/>
  </xs:choice></xs:complexType></xs:element>
</xs:schema>"#;

const FACETED: &str = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:element name="r"><xs:complexType><xs:sequence>
    <xs:element name="v" maxOccurs="unbounded">
      <xs:simpleType><xs:restriction base="xs:string">
        <xs:pattern value="[a-z]{3}-[0-9]{4}"/>
        <xs:minLength value="8"/>
      </xs:restriction></xs:simpleType>
    </xs:element>
  </xs:sequence></xs:complexType></xs:element>
</xs:schema>"#;

fn items(n: usize) -> String {
    let mut s = String::from("<r>");
    for i in 0..n {
        let _ = write!(s, r#"<item id="{i}"><name>n{i}</name></item>"#);
    }
    s.push_str("</r>");
    s
}

fn mixed(n: usize) -> String {
    let mut s = String::from("<r>");
    for i in 0..n {
        match i % 3 {
            0 => s.push_str("<a>x</a>"),
            1 => s.push_str("<b>1</b>"),
            _ => s.push_str("<c>2001-01-01</c>"),
        }
    }
    s.push_str("</r>");
    s
}

fn codes(n: usize) -> String {
    let mut s = String::from("<r>");
    for i in 0..n {
        let _ = write!(s, "<v>abc-{:04}</v>", i % 10_000);
    }
    s.push_str("</r>");
    s
}

fn bench(c: &mut Criterion) {
    let cases: [(&str, &str, String); 3] = [
        ("sequence_1000", SEQUENCE, items(1000)),
        ("choice_1000", CHOICE, mixed(1000)),
        ("faceted_1000", FACETED, codes(1000)),
    ];

    let mut group = c.benchmark_group("validate");
    for (name, xsd, xml) in &cases {
        let schema = xmlschema::parse_schema(xsd).expect("schema parses");
        let doc = oxml::parse(xml).expect("well-formed");
        let _ = group.bench_function(*name, |b| {
            b.iter(|| xmlschema::validate(black_box(&doc), black_box(&schema)));
        });
    }
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
