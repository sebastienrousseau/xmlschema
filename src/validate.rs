// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 xmlschema. All rights reserved.

//! Validating a document against a [`Schema`].

use oxml::{Document, NodeId, NodeKind};

use crate::model::{BuiltIn, Content, Facets, Particle, Schema, SimpleType};
use crate::pattern::Pattern;

/// One validation failure.
///
/// Every violation carries the path to the offending element. Callers
/// almost always want to fix everything in one pass rather than
/// re-running the validator after each fix, which is why validation
/// collects rather than short-circuits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    /// A slash-separated path, e.g. `/library/book[2]/title`.
    pub path: String,
    /// What is wrong, in a form a person can act on.
    pub message: String,
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.path, self.message)
    }
}

/// The outcome of validating a document.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// Every violation found, in document order.
    pub violations: Vec<Violation>,
}

impl Report {
    /// Whether the document is valid.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.violations.is_empty()
    }
}

impl std::fmt::Display for Report {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.violations.is_empty() {
            return f.write_str("valid");
        }
        for (i, v) in self.violations.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "{v}")?;
        }
        Ok(())
    }
}

struct Validator<'a> {
    doc: &'a Document,
    report: Report,
}

/// Validate a parsed document against a schema.
#[must_use]
pub fn validate(doc: &Document, schema: &Schema) -> Report {
    let mut v = Validator {
        doc,
        report: Report::default(),
    };

    let Some(root) = doc.root_element() else {
        v.report.violations.push(Violation {
            path: "/".to_owned(),
            message: "the document has no root element".to_owned(),
        });
        return v.report;
    };

    let name = doc
        .element_name(root)
        .map(|n| n.local.clone())
        .unwrap_or_default();

    let Some(decl) = schema.element(&name) else {
        v.report.violations.push(Violation {
            path: format!("/{name}"),
            message: format!(
                "the schema declares no top-level element named `{name}` \
                 (it declares: {})",
                schema
                    .elements
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        });
        return v.report;
    };

    v.check_element(root, decl, &format!("/{name}"));
    v.report
}

impl Validator<'_> {
    fn check_element(&mut self, node: NodeId, decl: &Particle, path: &str) {
        self.check_attributes(node, decl, path);

        match &*decl.content {
            Content::Any => {}
            Content::Empty => {
                if self.doc.children(node).iter().any(|&c| {
                    self.doc.is_element(c)
                        || matches!(
                            self.doc.kind(c),
                            Some(NodeKind::Text(t)) if !t.trim().is_empty()
                        )
                }) {
                    self.violate(path, "this element must be empty".to_owned());
                }
            }
            Content::Simple(st) => {
                let text = self.doc.text(node);
                if self
                    .doc
                    .children(node)
                    .iter()
                    .any(|&c| self.doc.is_element(c))
                {
                    self.violate(
                        path,
                        "this element must contain text, not child \
                         elements"
                            .to_owned(),
                    );
                } else if let Err(why) = check_simple(&text, st) {
                    self.violate(path, why);
                }
            }
            Content::Sequence(particles) => {
                self.check_sequence(node, particles, path);
            }
            Content::Choice(branches) => {
                self.check_choice(node, branches, path);
            }
        }
    }

    /// Child elements must appear in the declared order, each within
    /// its cardinality.
    ///
    /// The walk is positional rather than counting-then-comparing,
    /// because `<a/><b/><a/>` against `sequence(a, b)` is an *ordering*
    /// error, and a counting check would report it as "too many a"
    /// which is not what went wrong.
    fn check_sequence(
        &mut self,
        node: NodeId,
        particles: &[Particle],
        path: &str,
    ) {
        let kids: Vec<NodeId> = self
            .doc
            .children(node)
            .iter()
            .copied()
            .filter(|&c| self.doc.is_element(c))
            .collect();

        let mut index = 0usize;
        for particle in particles {
            let mut seen = 0usize;
            while index < kids.len() {
                let child = kids[index];
                let child_name = self
                    .doc
                    .element_name(child)
                    .map(|n| n.local.as_str())
                    .unwrap_or_default();
                if child_name != particle.name {
                    break;
                }
                if particle.occurs.max.is_some_and(|m| seen >= m) {
                    break;
                }
                seen += 1;
                let child_path = if particle.occurs.max == Some(1) {
                    format!("{path}/{child_name}")
                } else {
                    format!("{path}/{child_name}[{seen}]")
                };
                self.check_element(child, particle, &child_path);
                index += 1;
            }
            if !particle.occurs.permits(seen) {
                self.violate(
                    path,
                    format!(
                        "expected `{}` {}, found {seen}",
                        particle.name,
                        particle.occurs.describe()
                    ),
                );
            }
        }

        for &extra in kids.iter().skip(index) {
            let extra_name = self
                .doc
                .element_name(extra)
                .map(|n| n.local.as_str())
                .unwrap_or_default();
            let expected: Vec<&str> =
                particles.iter().map(|p| p.name.as_str()).collect();
            self.violate(
                &format!("{path}/{extra_name}"),
                format!(
                    "unexpected element `{extra_name}`; this content \
                     model allows {} in that order",
                    expected.join(", ")
                ),
            );
        }
    }

    fn check_choice(
        &mut self,
        node: NodeId,
        branches: &[Particle],
        path: &str,
    ) {
        let kids: Vec<NodeId> = self
            .doc
            .children(node)
            .iter()
            .copied()
            .filter(|&c| self.doc.is_element(c))
            .collect();

        for &child in &kids {
            let child_name = self
                .doc
                .element_name(child)
                .map(|n| n.local.clone())
                .unwrap_or_default();
            if let Some(branch) = branches.iter().find(|b| b.name == child_name)
            {
                self.check_element(
                    child,
                    branch,
                    &format!("{path}/{child_name}"),
                );
            } else {
                let allowed: Vec<&str> =
                    branches.iter().map(|b| b.name.as_str()).collect();
                self.violate(
                    &format!("{path}/{child_name}"),
                    format!(
                        "`{child_name}` is not one of the permitted \
                         choices ({})",
                        allowed.join(", ")
                    ),
                );
            }
        }
    }

    fn check_attributes(&mut self, node: NodeId, decl: &Particle, path: &str) {
        for want in &decl.attributes {
            match self.doc.attribute(node, &want.name) {
                Some(value) => {
                    if let Err(why) = check_simple(value, &want.simple_type) {
                        self.violate(&format!("{path}/@{}", want.name), why);
                    }
                }
                None if want.required => {
                    self.violate(
                        path,
                        format!("missing required attribute `{}`", want.name),
                    );
                }
                None => {}
            }
        }
    }

    fn violate(&mut self, path: &str, message: String) {
        self.report.violations.push(Violation {
            path: path.to_owned(),
            message,
        });
    }
}

/// Check a text value against a simple type.
fn check_simple(value: &str, st: &SimpleType) -> Result<(), String> {
    check_builtin(value, st.base)?;
    check_facets(value, &st.facets, st.base)
}

fn check_builtin(value: &str, base: BuiltIn) -> Result<(), String> {
    let v = value.trim();
    let ok = match base {
        BuiltIn::String | BuiltIn::AnyUri => true,
        BuiltIn::Boolean => {
            matches!(v, "true" | "false" | "1" | "0")
        }
        BuiltIn::Decimal | BuiltIn::Double => v.parse::<f64>().is_ok(),
        BuiltIn::Integer => v.parse::<i64>().is_ok(),
        BuiltIn::NonNegativeInteger => v.parse::<u64>().is_ok(),
        BuiltIn::Date => is_date(v),
        BuiltIn::DateTime => {
            // Split at the mandatory `T`; the time half is checked
            // loosely because timezone forms are numerous and the
            // failure mode of accepting an odd offset is far milder
            // than rejecting a valid one.
            v.split_once('T').is_some_and(|(d, t)| {
                is_date(d) && t.len() >= 8 && t.as_bytes()[2] == b':'
            })
        }
    };
    if ok {
        Ok(())
    } else {
        Err(format!(
            "`{value}` is not a valid {}",
            describe_builtin(base)
        ))
    }
}

fn describe_builtin(base: BuiltIn) -> &'static str {
    match base {
        BuiltIn::String => "string",
        BuiltIn::Boolean => "boolean (true, false, 1 or 0)",
        BuiltIn::Decimal => "decimal",
        BuiltIn::Integer => "integer",
        BuiltIn::NonNegativeInteger => "non-negative integer",
        BuiltIn::Double => "double",
        BuiltIn::Date => "date (YYYY-MM-DD)",
        BuiltIn::DateTime => "dateTime (YYYY-MM-DDThh:mm:ss)",
        BuiltIn::AnyUri => "URI",
    }
}

/// `YYYY-MM-DD`, with real range checks on the parts.
fn is_date(v: &str) -> bool {
    let parts: Vec<&str> = v.split('-').collect();
    // A leading `-` makes a BCE year, so the split yields an empty
    // first part; both shapes carry the same three fields.
    let (y, m, d) = match parts.as_slice() {
        [y, m, d] | ["", y, m, d] => (*y, *m, *d),
        _ => return false,
    };
    if y.len() < 4 || m.len() != 2 || d.len() != 2 {
        return false;
    }
    let (Ok(_year), Ok(month), Ok(day)) = (
        y.parse::<i32>(),
        m.parse::<u32>(),
        // The day may carry a timezone suffix on dateTime's date half.
        d.get(..2).unwrap_or(d).parse::<u32>(),
    ) else {
        return false;
    };
    (1..=12).contains(&month) && (1..=31).contains(&day)
}

fn check_facets(
    value: &str,
    facets: &Facets,
    base: BuiltIn,
) -> Result<(), String> {
    if facets.is_empty() {
        return Ok(());
    }

    if !facets.enumeration.is_empty()
        && !facets.enumeration.iter().any(|e| e == value)
    {
        return Err(format!(
            "`{value}` is not one of the permitted values ({})",
            facets.enumeration.join(", ")
        ));
    }

    let len = value.chars().count();
    if let Some(want) = facets.length {
        if len != want {
            return Err(format!(
                "`{value}` must be exactly {want} characters, not {len}"
            ));
        }
    }
    if let Some(min) = facets.min_length {
        if len < min {
            return Err(format!(
                "`{value}` must be at least {min} characters, not {len}"
            ));
        }
    }
    if let Some(max) = facets.max_length {
        if len > max {
            return Err(format!(
                "`{value}` must be at most {max} characters, not {len}"
            ));
        }
    }

    if let Some(source) = &facets.pattern {
        match Pattern::compile(source) {
            Ok(p) => {
                if !p.matches(value) {
                    return Err(format!(
                        "`{value}` does not match the pattern `{source}`"
                    ));
                }
            }
            Err(e) => {
                return Err(format!(
                    "the schema's pattern `{source}` could not be \
                     compiled: {e}"
                ));
            }
        }
    }

    // Numeric bounds only mean something for numeric bases.
    if matches!(
        base,
        BuiltIn::Decimal
            | BuiltIn::Double
            | BuiltIn::Integer
            | BuiltIn::NonNegativeInteger
    ) {
        if let Ok(n) = value.trim().parse::<f64>() {
            if let Some(b) = facets.min_inclusive {
                if n < b {
                    return Err(format!("{value} must be at least {b}"));
                }
            }
            if let Some(b) = facets.max_inclusive {
                if n > b {
                    return Err(format!("{value} must be at most {b}"));
                }
            }
            if let Some(b) = facets.min_exclusive {
                if n <= b {
                    return Err(format!("{value} must be greater than {b}"));
                }
            }
            if let Some(b) = facets.max_exclusive {
                if n >= b {
                    return Err(format!("{value} must be less than {b}"));
                }
            }
        }
    }

    Ok(())
}
