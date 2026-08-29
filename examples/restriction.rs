// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 xmlschema. All rights reserved.

//! Deciding whether one content model restricts another.
//!
//! Run with:
//!
//! ```text
//! cargo run --example restriction
//! ```
//!
//! A complex type derived by restriction may only accept documents its
//! base would also accept. This is the machinery that decides that,
//! and the reporting that says when the answer cannot be trusted.

use xmlschema::derive::{
    Compositor, Particle, Term, is_valid_restriction, namespace_subset,
    particle_of,
};
use xmlschema::support::unsupported;

/// A schema with a two-particle sequence, read back below.
const SCHEMA: &str = r#"<?xml version="1.0"?>
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:complexType name="book">
    <xs:sequence>
      <xs:element name="title" type="xs:string"/>
      <xs:element name="year" type="xs:integer" minOccurs="0"/>
    </xs:sequence>
  </xs:complexType>
</xs:schema>"#;

fn element(name: &str, min: usize, max: Option<usize>) -> Particle {
    Particle {
        min,
        max,
        term: Term::Element {
            name: name.to_owned(),
            type_name: Some("xs:string".to_owned()),
        },
    }
}

fn main() {
    // A group of one occurring exactly once *is* the particle it
    // contains -- the specification calls this eliminating a pointless
    // particle, and it has to happen before any rule is applied.
    let wrapped = Particle {
        min: 1,
        max: Some(1),
        term: Term::Group {
            compositor: Compositor::Sequence,
            particles: vec![element("title", 1, Some(1))],
        },
    };
    println!("collapsed: {:?}", wrapped.collapsed().term);
    assert!(matches!(wrapped.collapsed().term, Term::Element { .. }));

    // Counting what is inside: three elements in a sequence that
    // occurs twice can match six times.
    let group = Particle {
        min: 2,
        max: Some(2),
        term: Term::Group {
            compositor: Compositor::Sequence,
            particles: vec![
                element("a", 1, Some(1)),
                element("b", 1, Some(1)),
                element("c", 1, Some(1)),
            ],
        },
    };
    println!("effective total range: {:?}", group.effective_total_range());
    assert_eq!(group.effective_total_range(), (6, Some(6)));

    // Whether a particle can match nothing at all.
    assert!(!group.emptiable(), "every member is required");
    assert!(element("title", 0, Some(1)).emptiable());

    // Narrowing an occurrence range is a valid restriction; widening
    // one is not.
    let base = element("title", 0, None);
    let narrowed = element("title", 1, Some(1));
    let same_type = |_derived: &str, _base: &str| true;

    println!(
        "0..* restricted to 1..1: {}",
        is_valid_restriction(&narrowed, &base, &same_type)
    );
    assert!(is_valid_restriction(&narrowed, &base, &same_type));
    assert!(
        !is_valid_restriction(&base, &narrowed, &same_type),
        "widening 1..1 to 0..* accepts documents the base rejects"
    );

    // Wildcards narrow by namespace.
    assert!(namespace_subset("urn:example", "##any"));
    assert!(!namespace_subset("##any", "urn:example"));
    assert!(namespace_subset("urn:a urn:b", "urn:a urn:b urn:c"));
    assert!(!namespace_subset("urn:z", "urn:a urn:b"));

    // Reading a model group out of a schema document.

    let doc = oxml::parse(SCHEMA).expect("the schema is well-formed");
    let sequence = doc
        .descendants()
        .find(|id| {
            doc.is_element(*id)
                && doc.element_name(*id).is_some_and(|n| n.local == "sequence")
        })
        .expect("the schema has a sequence");

    let no_groups = |_name: &str| None;
    let particle =
        particle_of(&doc, sequence, &no_groups, 0).expect("a model group");
    println!(
        "read from the schema: {:?}",
        particle.effective_total_range()
    );
    assert_eq!(
        particle.effective_total_range(),
        (1, Some(2)),
        "title is required and year is optional"
    );

    // Everything this crate does not enforce. An empty result means a
    // validation outcome can be taken at face value; a non-empty one
    // means something was skipped.
    let gaps = unsupported(&doc);
    println!("unsupported constructs: {}", gaps.len());
    assert!(gaps.is_empty(), "this schema uses nothing unsupported");

    // `<xs:sequence maxOccurs="2">` repeats the *group*: `(a, b){2}`
    // permits `a b a b` and not `a a b b`. This crate has no
    // repeated-group model, so it says so rather than quietly
    // enforcing the weaker constraint.
    let repeated = oxml::parse(
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
             <xs:complexType name="pairs">
               <xs:sequence maxOccurs="2">
                 <xs:element name="a" type="xs:string"/>
                 <xs:element name="b" type="xs:string"/>
               </xs:sequence>
             </xs:complexType>
           </xs:schema>"#,
    )
    .expect("well-formed");
    let reported = unsupported(&repeated);
    for gap in &reported {
        println!("{}: {}", gap.construct, gap.effect);
    }
    assert!(
        !reported.is_empty(),
        "an unenforced construct must be reported, not silently skipped"
    );
}
