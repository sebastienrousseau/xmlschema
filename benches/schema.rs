// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 xmlschema. All rights reserved.

//! Reading an `.xsd` into a `Schema`.
//!
//! Schemas are parsed once and used many times, so this is the cost a
//! caller pays at startup. It is also where the sharp edges are: a
//! referenced type is *inlined* at every use, so the shapes that
//! matter are the ones that reuse types rather than the ones that are
//! merely large.

#![allow(missing_docs)] // criterion_group! generates an undocumented item

use core::fmt::Write as _;
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

/// A schema with `n` independent top-level elements.
fn wide(n: usize) -> String {
    let mut s = String::from(
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">"#,
    );
    for i in 0..n {
        let _ = write!(
            s,
            r#"<xs:element name="e{i}">
                 <xs:complexType><xs:sequence>
                   <xs:element name="a" type="xs:string"/>
                   <xs:element name="b" type="xs:integer"/>
                 </xs:sequence>
                 <xs:attribute name="k" type="xs:date"/>
                 </xs:complexType>
               </xs:element>"#
        );
    }
    s.push_str("</xs:schema>");
    s
}

/// A chain of named types, each referencing the previous once.
///
/// Linear in the number of levels only because parsing is memoised;
/// without that it is quadratic in the work and exponential in the
/// tree.
fn chained(levels: usize) -> String {
    let mut s = String::from(
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
             <xs:complexType name="t0">
               <xs:sequence><xs:element name="leaf" type="xs:string"/></xs:sequence>
             </xs:complexType>"#,
    );
    for i in 1..=levels {
        let prev = i - 1;
        let _ = write!(
            s,
            r#"<xs:complexType name="t{i}"><xs:sequence>
                 <xs:element name="a" type="t{prev}"/>
               </xs:sequence></xs:complexType>"#
        );
    }
    let _ = write!(s, r#"<xs:element name="r" type="t{levels}"/>"#);
    s.push_str("</xs:schema>");
    s
}

/// A simple type with many facets, which is the other common shape.
fn faceted(n: usize) -> String {
    let mut s = String::from(
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
             <xs:element name="r">
               <xs:simpleType><xs:restriction base="xs:string">"#,
    );
    for i in 0..n {
        let _ = write!(s, r#"<xs:enumeration value="v{i}"/>"#);
    }
    s.push_str("</xs:restriction></xs:simpleType></xs:element></xs:schema>");
    s
}

fn bench(c: &mut Criterion) {
    let wide_doc = wide(200);
    let chained_doc = chained(50);
    let faceted_doc = faceted(500);

    let mut group = c.benchmark_group("schema");
    let _ = group.bench_function("wide_200_elements", |b| {
        b.iter(|| xmlschema::parse_schema(black_box(&wide_doc)).unwrap());
    });
    let _ = group.bench_function("chained_50_types", |b| {
        b.iter(|| xmlschema::parse_schema(black_box(&chained_doc)).unwrap());
    });
    let _ = group.bench_function("500_enumeration_facets", |b| {
        b.iter(|| xmlschema::parse_schema(black_box(&faceted_doc)).unwrap());
    });
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
