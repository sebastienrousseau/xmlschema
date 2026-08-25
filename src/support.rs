// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 xmlschema. All rights reserved.

//! What this crate does *not* enforce in a given schema.
//!
//! This crate implements a subset of XSD 1.0 and, by design, skips a
//! construct it does not understand rather than rejecting the schema:
//! the surrounding rules still apply, so a schema using `xs:all`
//! validates everything else correctly instead of failing wholesale.
//!
//! That is the right behaviour for a user and the wrong behaviour for
//! a *measurement*. A schema whose constraints were all skipped
//! accepts every document, so a conformance test expecting "valid"
//! agrees with it — and counting that as a pass reports enforcement
//! that never happened. Across a suite of forty thousand tests, that
//! is the difference between a number and a flattering number.
//!
//! So this module answers the question the validator cannot: **given
//! this schema, what stops being checked?** It is deliberately
//! independent of the parser rather than instrumented into it, for two
//! reasons. It cannot fall out of step with a degradation site it does
//! not know about, because it works from a whitelist of what *is*
//! enforced and reports everything else. And every uncertainty
//! resolves toward "not enforced", which can only ever lower a
//! reported pass rate.

use oxml::{Document, NodeId};

/// A construct present in the schema that this crate does not enforce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unsupported {
    /// The construct, as it appears in the schema — `xs:union`,
    /// `xs:byte`, `xs:totalDigits`.
    pub construct: String,
    /// What stops being checked because of it.
    pub effect: String,
}

/// Elements this crate reads and acts on.
///
/// `annotation`, `documentation` and `appinfo` carry no constraints,
/// so ignoring them enforces nothing less.
const HANDLED_ELEMENTS: &[&str] = &[
    "schema",
    "list",
    "union",
    "element",
    "complexType",
    "simpleType",
    "sequence",
    "choice",
    "restriction",
    "attribute",
    "simpleContent",
    "extension",
    "annotation",
    "documentation",
    "appinfo",
    "all",
    "group",
    "attributeGroup",
    "complexContent",
    "any",
    "anyAttribute",
    // Facets, handled below by name.
    "enumeration",
    "pattern",
    "minLength",
    "maxLength",
    "length",
    "minInclusive",
    "maxInclusive",
    "minExclusive",
    "maxExclusive",
    "totalDigits",
    "fractionDigits",
    "whiteSpace",
];

/// Every construct in `doc` that this crate does not enforce.
///
/// An empty result means the schema is enforced in full, so a
/// validation outcome for it can be taken at face value. A non-empty
/// one means at least one constraint was skipped, and a document this
/// crate calls valid may not be.
///
/// Takes the schema as a parsed [`Document`] rather than as source so
/// a caller that already parsed it does not parse it twice.
#[must_use]
pub fn unsupported(doc: &Document) -> Vec<Unsupported> {
    let mut out = Vec::new();
    for id in doc.descendants() {
        if !doc.is_element(id) {
            continue;
        }
        let Some(name) = doc.element_name(id).map(|n| n.local.clone()) else {
            continue;
        };
        check_element(doc, id, &name, &mut out);
    }
    out.sort_by(|a, b| a.construct.cmp(&b.construct));
    out.dedup();
    out
}

fn check_element(
    doc: &Document,
    id: NodeId,
    name: &str,
    out: &mut Vec<Unsupported>,
) {
    if !HANDLED_ELEMENTS.contains(&name) {
        out.push(Unsupported {
            construct: format!("xs:{name}"),
            effect: "the element is ignored entirely".to_owned(),
        });
        return;
    }

    // A `sequence` or `choice` yields only its *element* children, so
    // a nested model group inside one is dropped along with every
    // constraint it carries.
    // `<xs:sequence maxOccurs="2">` repeats the *group*: `(a, b){2}`
    // permits `a b a b` and not `a a b b`. With one particle that is
    // the same as multiplying its own cardinality, which the parser
    // does; with more than one it is not, and this crate has no
    // repeated-group model.
    if matches!(name, "sequence" | "choice" | "all") {
        let repeats = doc.attribute(id, "maxOccurs").is_some_and(|v| v != "1")
            || doc.attribute(id, "minOccurs").is_some_and(|v| v != "1");
        let particles = doc
            .children(id)
            .iter()
            .filter(|&&c| {
                doc.element_name(c).is_some_and(|n| n.local != "annotation")
            })
            .count();
        if repeats && particles > 1 {
            out.push(Unsupported {
                construct: format!("repeated xs:{name}"),
                effect: "a model group repeated as a whole is not \
                         modelled, so the order across repetitions is \
                         not enforced"
                    .to_owned(),
            });
        }
    }

    // A pattern this engine cannot compile constrains nothing. The
    // validator reports it as a violation, which turns an
    // unsupported *regex* into an invalid *document* -- the wrong
    // answer, and one that counts against the pass rate as though the
    // document were at fault.
    if name == "pattern" {
        if let Some(value) = doc.attribute(id, "value") {
            if let Err(why) = crate::pattern::Pattern::compile(value) {
                out.push(Unsupported {
                    construct: format!("xs:pattern {value:?}"),
                    effect: format!("the pattern does not compile: {why}"),
                });
            }
        }
    }

    // A type reference is resolved against the built-ins and this
    // schema's named simple types; anything else becomes "accepts
    // anything".
    if let Some(type_name) = doc.attribute(id, "type") {
        check_type_reference(doc, type_name, out);
    }
    if name == "restriction" || name == "extension" {
        if let Some(base) = doc.attribute(id, "base") {
            check_type_reference(doc, base, out);
        }
    }

    // Attributes that change what a declaration means, none of which
    // this crate reads.
    for (attribute, effect) in [
        ("abstract", "the declaration is used as if it were concrete"),
        ("substitutionGroup", "substitution is not applied"),
        ("default", "the default value is not supplied"),
        ("form", "qualification is not applied"),
        ("block", "the blocking constraint is not applied"),
        ("final", "the derivation constraint is not applied"),
        (
            "mixed",
            "mixed content is not distinguished from element-only",
        ),
    ] {
        if doc.attribute(id, attribute).is_some() {
            out.push(Unsupported {
                construct: format!("@{attribute} on xs:{name}"),
                effect: (*effect).to_owned(),
            });
        }
    }
}

/// A type reference that resolves to nothing constrains nothing.
fn check_type_reference(
    doc: &Document,
    type_name: &str,
    out: &mut Vec<Unsupported>,
) {
    let local = type_name.rsplit(':').next().unwrap_or(type_name);
    if oxml_builtin(local) || names_a_local_type(doc, local) {
        return;
    }
    out.push(Unsupported {
        construct: format!("type reference {type_name:?}"),
        effect: "resolves to nothing, so the element accepts any content"
            .to_owned(),
    });
}

/// The built-ins carried at full strength, which is now all of them:
/// [`crate::datatype`] models every XSD 1.0 built-in separately, so a
/// name it resolves is a name this crate enforces.
fn oxml_builtin(local: &str) -> bool {
    crate::datatype::Datatype::from_name(local).is_some()
}

/// Whether the schema declares a named type with this local name.
fn names_a_local_type(doc: &Document, local: &str) -> bool {
    doc.descendants().any(|id| {
        doc.element_name(id).is_some_and(|n| {
            (n.local == "simpleType" || n.local == "complexType")
                && doc.attribute(id, "name") == Some(local)
        })
    })
}
