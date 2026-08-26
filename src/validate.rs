// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 xmlschema. All rights reserved.

//! Validating a document against a [`Schema`].

use oxml::{Document, NodeId, NodeKind};

use crate::datatype::WhiteSpace;
use oxml::XPath;
use std::collections::BTreeMap;

use crate::model::{
    BuiltIn, Content, Facets, IdentityKind, NamespaceConstraint, Particle,
    ProcessContents, Schema, SimpleType, Variety,
};

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

/// The XML Schema instance namespace, whose attributes -- `xsi:nil`,
/// `xsi:type` and the two `schemaLocation`s -- are defined by the
/// specification rather than by the schema being validated against.
const XSI: &str = "http://www.w3.org/2001/XMLSchema-instance";

struct Validator<'a> {
    doc: &'a Document,
    schema: &'a Schema,
    /// Field tuples gathered per identity constraint name.
    ///
    /// A `keyref` must be checked against the constraint it refers to,
    /// which may be declared on a different element and reached later
    /// in the walk -- so tuples are collected as they are found and
    /// the references checked once at the end.
    identities: BTreeMap<String, Vec<Vec<String>>>,
    /// Every `keyref` seen, with the tuples it must find.
    references: Vec<(String, String, Vec<Vec<String>>, String)>,
    /// The schema's target namespace, which `##other` is defined
    /// against.
    target: Option<String>,
    report: Report,
}

/// Validate a parsed document against a schema.
#[must_use]
pub fn validate(doc: &Document, schema: &Schema) -> Report {
    let mut v = Validator {
        doc,
        schema,
        identities: BTreeMap::new(),
        references: Vec::new(),
        target: schema.target_namespace.clone(),
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
    v.check_references();
    v.report
}

impl Validator<'_> {
    fn check_element(&mut self, node: NodeId, decl: &Particle, path: &str) {
        self.check_attributes(node, decl, path);
        if !decl.identities.is_empty() {
            self.check_identities(node, decl, path);
        }

        // `xsi:nil="true"` stands in for content, but only where the
        // declaration allows it. Where it does, the element must be
        // empty and its content model is not applied.
        let nil = self
            .doc
            .attribute(node, "nil")
            .or_else(|| self.doc.attribute(node, "xsi:nil"))
            == Some("true");
        if nil {
            if !decl.nillable {
                self.violate(path, "this element is not nillable".to_owned());
            } else if !self.doc.text(node).trim().is_empty() {
                self.violate(path, "a nilled element must be empty".to_owned());
            }
            return;
        }

        // A fixed value constrains the element's character content
        // exactly, whatever else its type permits.
        if let Some(fixed) = decl.fixed.as_deref() {
            let text = self.doc.text(node);
            if text.trim() != fixed {
                self.violate(
                    path,
                    format!(
                        "must be the fixed value `{fixed}`, not `{}`",
                        text.trim()
                    ),
                );
            }
        }

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
            Content::All(particles) => {
                self.check_all(node, particles, path);
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
                if !self.particle_matches(child, particle, child_name) {
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
                self.check_matched(child, particle, &child_path);
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
            // Matched through `particle_matches`, so a wildcard branch
            // is considered. Comparing names alone never matched one:
            // a wildcard particle has no name, which is also why the
            // diagnostic used to list the permitted choices as `()`.
            let branch = branches
                .iter()
                .position(|b| self.particle_matches(child, b, &child_name));
            if let Some(index) = branch {
                let particle = branches[index].clone();
                self.check_matched(
                    child,
                    &particle,
                    &format!("{path}/{child_name}"),
                );
            } else {
                let allowed: Vec<&str> = branches
                    .iter()
                    .map(|b| {
                        if b.wildcard.is_some() {
                            "any"
                        } else {
                            b.name.as_str()
                        }
                    })
                    .collect();
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
        // An `xs:anyAttribute` admits attributes the type does not
        // declare. Without it, undeclared attributes are simply not
        // reported by this validator either way -- but with a strict
        // one they must have a top-level declaration.
        if let Some(wildcard) = decl.any_attribute.as_ref() {
            if wildcard.process == ProcessContents::Strict {
                for &attr in self.doc.attribute_nodes(node) {
                    let Some(NodeKind::Attr(a)) = self.doc.kind(attr) else {
                        continue;
                    };
                    let Some(name) = self.doc.name(a.name) else {
                        continue;
                    };
                    let local = name.local.clone();
                    // `xsi:` attributes are defined by the schema
                    // instance namespace, not by this schema.
                    if name.namespace.as_deref() == Some(XSI) {
                        continue;
                    }
                    if decl.attributes.iter().all(|d| d.name != local) {
                        self.violate(
                            &format!("{path}/@{local}"),
                            "matched a strict wildcard but has no \
                             declaration"
                                .to_owned(),
                        );
                    }
                }
            }
        }

        for want in &decl.attributes {
            match self.doc.attribute(node, &want.name) {
                Some(_) if want.prohibited => {
                    self.violate(
                        path,
                        format!("attribute `{}` is prohibited here", want.name),
                    );
                }
                Some(value) => {
                    if let Err(why) = check_simple(value, &want.simple_type) {
                        self.violate(&format!("{path}/@{}", want.name), why);
                    }
                    // A fixed value is not a default: if the attribute
                    // appears at all, it must be exactly this.
                    if let Some(fixed) = want.fixed.as_deref() {
                        if value != fixed {
                            self.violate(
                                &format!("{path}/@{}", want.name),
                                format!(
                                    "must be the fixed value `{fixed}`, \
                                     not `{value}`"
                                ),
                            );
                        }
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

    /// `xs:all` — every declared child may appear in any order, and
    /// each at most once.
    ///
    /// Not a sequence with the ordering relaxed: validating it as one
    /// rejects a document whose children are simply in a different
    /// order, which is the entire point of the construct.
    fn check_all(&mut self, node: NodeId, particles: &[Particle], path: &str) {
        let kids: Vec<NodeId> = self
            .doc
            .children(node)
            .iter()
            .copied()
            .filter(|&c| self.doc.is_element(c))
            .collect();

        let mut counts: Vec<usize> = vec![0; particles.len()];
        for child in kids {
            let name = self
                .doc
                .element_name(child)
                .map(|n| n.local.clone())
                .unwrap_or_default();
            match particles
                .iter()
                .position(|p| self.particle_matches(child, p, &name))
            {
                Some(index) => {
                    counts[index] += 1;
                    if counts[index] > 1 {
                        self.violate(
                            path,
                            format!("`{name}` appears more than once"),
                        );
                    } else {
                        let child_path = format!("{path}/{name}");
                        let particle = particles[index].clone();
                        self.check_matched(child, &particle, &child_path);
                    }
                }
                None => self
                    .violate(path, format!("`{name}` is not permitted here")),
            }
        }
        for (particle, seen) in particles.iter().zip(&counts) {
            if *seen == 0 && particle.occurs.min > 0 {
                self.violate(
                    path,
                    format!("missing required `{}`", particle.name),
                );
            }
        }
    }

    /// Whether `child` is what `particle` declares.
    ///
    /// A named particle matches by local name; a wildcard matches by
    /// namespace, which is the whole reason it exists.
    fn particle_matches(
        &self,
        child: NodeId,
        particle: &Particle,
        child_name: &str,
    ) -> bool {
        let Some(wildcard) = particle.wildcard.as_ref() else {
            return child_name == particle.name;
        };
        let namespace = self
            .doc
            .element_name(child)
            .and_then(|n| n.namespace.clone());
        match &wildcard.namespaces {
            NamespaceConstraint::Any => true,
            // `##other` is anything *outside* the target namespace,
            // and an unqualified element is outside every namespace.
            NamespaceConstraint::Other => {
                namespace.as_deref() != self.target.as_deref()
            }
            NamespaceConstraint::List(allowed) => {
                allowed.iter().any(|a| a.as_deref() == namespace.as_deref())
            }
        }
    }

    /// Validate a child that a particle matched.
    ///
    /// For a wildcard this depends on `processContents`: `skip`
    /// validates nothing, `lax` validates only if the element has a
    /// top-level declaration, and `strict` requires one.
    fn check_matched(
        &mut self,
        child: NodeId,
        particle: &Particle,
        path: &str,
    ) {
        let Some(wildcard) = particle.wildcard.as_ref() else {
            self.check_element(child, particle, path);
            return;
        };
        if wildcard.process == ProcessContents::Skip {
            return;
        }
        let name = self
            .doc
            .element_name(child)
            .map(|n| n.local.clone())
            .unwrap_or_default();
        match self.schema.elements.get(&name) {
            Some(decl) => {
                let decl = decl.clone();
                self.check_element(child, &decl, path);
            }
            None if wildcard.process == ProcessContents::Strict => {
                self.violate(
                    path,
                    format!(
                        "`{name}` matched a strict wildcard but has no \
                         declaration"
                    ),
                );
            }
            None => {}
        }
    }

    /// Evaluate the identity constraints declared on an element.
    ///
    /// The selector chooses the nodes constrained and each field
    /// contributes one component of a tuple. `oxml`'s `XPath` engine
    /// evaluates both: XSD permits a restricted subset of the
    /// language, and a superset engine runs it unchanged.
    fn check_identities(&mut self, node: NodeId, decl: &Particle, path: &str) {
        for identity in &decl.identities {
            let Ok(selector) = XPath::compile(&identity.selector) else {
                // An expression this engine cannot compile constrains
                // nothing; `support::unsupported` reports the schema
                // rather than the document being blamed.
                continue;
            };
            let selected = selector.evaluate_from(self.doc, node);
            let Some(nodes) = selected.nodes() else {
                continue;
            };

            let mut tuples: Vec<Vec<String>> = Vec::new();
            let mut incomplete = false;
            for &selected_node in nodes {
                let mut tuple = Vec::with_capacity(identity.fields.len());
                for field in &identity.fields {
                    let Ok(expression) = XPath::compile(field) else {
                        tuple.clear();
                        break;
                    };
                    let value =
                        expression.evaluate_from(self.doc, selected_node);
                    // A field selecting nothing means the node has no
                    // value for this component.
                    if value.nodes().is_some_and(<[NodeId]>::is_empty) {
                        incomplete = true;
                        tuple.clear();
                        break;
                    }
                    tuple.push(value.to_str(self.doc));
                }
                if tuple.len() == identity.fields.len() {
                    tuples.push(tuple);
                }
            }

            // `key` requires every selected node to have a full tuple;
            // `unique` and `keyref` simply ignore the ones that do not.
            if identity.kind == IdentityKind::Key && incomplete {
                self.violate(
                    path,
                    format!(
                        "`{}` is a key, so every selected element must \
                         have all its fields",
                        identity.name
                    ),
                );
            }

            if identity.kind == IdentityKind::KeyRef {
                if let Some(refer) = identity.refer.clone() {
                    self.references.push((
                        identity.name.clone(),
                        refer,
                        tuples,
                        path.to_owned(),
                    ));
                }
                continue;
            }

            // `unique` and `key` both forbid a repeated tuple.
            let mut seen: Vec<&Vec<String>> = Vec::new();
            let mut duplicate = None;
            for tuple in &tuples {
                if seen.contains(&tuple) {
                    duplicate = Some(tuple.join(", "));
                    break;
                }
                seen.push(tuple);
            }
            if let Some(values) = duplicate {
                self.violate(
                    path,
                    format!(
                        "`{}` requires distinct values, and ({values}) \
                         appears more than once",
                        identity.name
                    ),
                );
            }
            self.identities
                .entry(identity.name.clone())
                .or_default()
                .extend(tuples);
        }
    }

    /// Every `keyref` tuple must appear among the tuples of the
    /// constraint it refers to.
    ///
    /// Checked after the whole document is walked, because the
    /// constraint referred to may be declared on an element reached
    /// later.
    fn check_references(&mut self) {
        let references = std::mem::take(&mut self.references);
        for (name, refer, tuples, path) in references {
            let Some(known) = self.identities.get(&refer).cloned() else {
                // A reference to a constraint that does not exist is a
                // schema defect rather than a document one, and is not
                // reported against the document.
                continue;
            };
            for tuple in tuples {
                if !known.contains(&tuple) {
                    self.violate(
                        &path,
                        format!(
                            "`{name}` refers to `{refer}`, which has no \
                             ({}) ",
                            tuple.join(", ")
                        ),
                    );
                }
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
    match &st.variety {
        // `xs:NMTOKENS`, `xs:IDREFS` and `xs:ENTITIES` are list types
        // that happen to be built in. Their length facets count
        // *items*, as any other list's do; counting characters agreed
        // by accident on short values.
        Variety::Atomic if st.base.is_built_in_list() => {
            check_builtin(value, st.base)?;
            let item = SimpleType::atomic(st.base.item_type());
            check_list(value, &item, &st.facets)
        }
        Variety::Atomic => {
            // `whiteSpace` is applied before anything else, because it
            // decides what the value *is*. A restriction may only
            // narrow the base's own rule -- preserve to replace to
            // collapse -- so overriding it here is the whole effect.
            let normalised = match st.facets.white_space {
                Some(rule) => apply_white_space(value, rule),
                None => st.base.normalise(value).into_owned(),
            };
            check_builtin(&normalised, st.base)?;
            check_facets(&normalised, &st.facets, st.base)
        }
        Variety::List(item) => check_list(value, item, &st.facets),
        Variety::Union(members) => check_union(value, members, &st.facets),
    }
}

/// Every item must satisfy the item type.
///
/// Length facets count *items* here, not characters: `minLength` on a
/// list of three integers means three values, not three digits.
/// Applying the atomic rule would compare a character count against an
/// item count and agree by accident.
fn check_list(
    value: &str,
    item: &SimpleType,
    facets: &Facets,
) -> Result<(), String> {
    let items: Vec<&str> = value.split_whitespace().collect();
    for one in &items {
        check_simple(one, item)
            .map_err(|e| format!("`{one}` is not a valid list item: {e}"))?;
    }
    let n = items.len();
    if let Some(want) = facets.length {
        if n != want {
            return Err(format!("the list has {n} items, not {want}"));
        }
    }
    if let Some(min) = facets.min_length {
        if n < min {
            return Err(format!("the list has {n} items, fewer than {min}"));
        }
    }
    if let Some(max) = facets.max_length {
        if n > max {
            return Err(format!("the list has {n} items, more than {max}"));
        }
    }
    // A pattern on a list applies to the *whole* space-separated
    // lexical form, not to each item. It was not applied at all,
    // which left every `list-<type>-pattern` schema in the suite --
    // some five hundred tests -- accepting anything.
    if let Some(pattern) = facets.pattern.as_ref() {
        if !pattern.matches(value) {
            return Err(format!(
                "`{value}` does not match the pattern `{}`",
                pattern.source()
            ));
        }
    }

    // An enumeration on a list constrains the whole space-separated
    // value, not the individual items.
    if !facets.enumeration.is_empty() {
        let joined = items.join(" ");
        if !facets.enumeration.contains(&joined) {
            return Err(format!("`{value}` is not one of the permitted lists"));
        }
    }
    Ok(())
}

/// The value must satisfy at least one member type.
///
/// The union's own facets apply on top, so a restricted union both
/// matches a member and satisfies the restriction.
fn check_union(
    value: &str,
    members: &[SimpleType],
    facets: &Facets,
) -> Result<(), String> {
    let matched = members.iter().any(|m| check_simple(value, m).is_ok());
    if !matched {
        return Err(format!(
            "`{value}` matches none of the {} member types",
            members.len()
        ));
    }
    if facets.is_empty() {
        return Ok(());
    }
    // Facets on a union are checked as strings: the value space is
    // the union of its members', which have no single base type.
    check_facets(value, facets, BuiltIn::String)
}

/// Significant total and fraction digit counts of a decimal value.
///
/// Both are properties of the value, not the lexical form: leading
/// zeros in the whole part and trailing zeros in the fraction are not
/// significant, so `01.20` is two and one.
fn digit_counts(value: &str) -> (usize, usize) {
    let v = value.trim();
    let body = v.strip_prefix(['+', '-']).unwrap_or(v);
    let (whole, fraction) = body.split_once('.').unwrap_or((body, ""));
    let whole = whole.trim_start_matches('0');
    let fraction = fraction.trim_end_matches('0');
    // A value of zero has one significant digit, not none.
    let whole_len = whole.len();
    let total = if whole_len + fraction.len() == 0 {
        1
    } else {
        whole_len + fraction.len()
    };
    (total, fraction.len())
}

/// Apply an explicit `xs:whiteSpace` facet.
fn apply_white_space(value: &str, rule: WhiteSpace) -> String {
    match rule {
        WhiteSpace::Preserve => value.to_owned(),
        WhiteSpace::Replace => value.replace(['\t', '\n', '\r'], " "),
        WhiteSpace::Collapse => {
            value.split_whitespace().collect::<Vec<_>>().join(" ")
        }
    }
}

fn check_builtin(value: &str, base: BuiltIn) -> Result<(), String> {
    if base.accepts(value) {
        Ok(())
    } else {
        Err(format!("`{value}` is not a valid {}", base.describe()))
    }
}

/// Check a value against the facets narrowing its type.
///
/// Split into one function per family. They are independent -- an
/// enumeration says nothing about a length, and a digit count says
/// nothing about a bound -- and reading them together meant reading a
/// hundred lines to find the ten that applied.
fn check_facets(
    value: &str,
    facets: &Facets,
    base: BuiltIn,
) -> Result<(), String> {
    if facets.is_empty() {
        return Ok(());
    }
    check_enumeration(value, facets)?;
    check_length(value, facets)?;
    check_pattern(value, facets)?;
    check_digits(value, facets)?;
    check_bounds(value, facets, base)
}

/// `xs:enumeration` — the value must be one of a fixed set.
fn check_enumeration(value: &str, facets: &Facets) -> Result<(), String> {
    if facets.enumeration.is_empty()
        || facets.enumeration.iter().any(|e| e == value)
    {
        return Ok(());
    }
    Err(format!(
        "`{value}` is not one of the permitted values ({})",
        facets.enumeration.join(", ")
    ))
}

/// `xs:length` and its bounds, counted in characters.
///
/// Characters rather than bytes, so a multi-byte value is not
/// measured as longer than it is.
fn check_length(value: &str, facets: &Facets) -> Result<(), String> {
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
    Ok(())
}

/// `xs:pattern`, compiled when the schema was read.
fn check_pattern(value: &str, facets: &Facets) -> Result<(), String> {
    let Some(pattern) = facets.pattern.as_ref() else {
        return Ok(());
    };
    if pattern.matches(value) {
        return Ok(());
    }
    Err(format!(
        "`{value}` does not match the pattern `{}`",
        pattern.source()
    ))
}

/// `xs:totalDigits` and `xs:fractionDigits`.
///
/// Both count *significant* digits, which is a property of the value:
/// a sign, leading zeros and a trailing zero fraction do not count, so
/// `01.20` is two and one rather than four and two.
fn check_digits(value: &str, facets: &Facets) -> Result<(), String> {
    if facets.total_digits.is_none() && facets.fraction_digits.is_none() {
        return Ok(());
    }
    let (total, fraction) = digit_counts(value);
    if let Some(max) = facets.total_digits {
        if total > max {
            return Err(format!(
                "{value} has {total} significant digits, more than {max}"
            ));
        }
    }
    if let Some(max) = facets.fraction_digits {
        if fraction > max {
            return Err(format!(
                "{value} has {fraction} fraction digits, more than {max}"
            ));
        }
    }
    Ok(())
}

/// The four ordering bounds.
///
/// They apply to every *ordered* type, which includes the dates, times
/// and durations as well as the numbers. Both sides are lexical forms
/// of the same type, so both convert through the datatype and the
/// units cancel.
fn check_bounds(
    value: &str,
    facets: &Facets,
    base: BuiltIn,
) -> Result<(), String> {
    use std::cmp::Ordering;

    if !base.is_ordered() {
        return Ok(());
    }
    let against = |raw: &Option<String>| {
        raw.as_deref()
            .and_then(|b| base.compare(value, b).map(|o| (b.to_owned(), o)))
    };
    if let Some((text, Ordering::Less)) = against(&facets.min_inclusive) {
        return Err(format!("{value} must be at least {text}"));
    }
    if let Some((text, Ordering::Greater)) = against(&facets.max_inclusive) {
        return Err(format!("{value} must be at most {text}"));
    }
    if let Some((text, Ordering::Less | Ordering::Equal)) =
        against(&facets.min_exclusive)
    {
        return Err(format!("{value} must be greater than {text}"));
    }
    if let Some((text, Ordering::Greater | Ordering::Equal)) =
        against(&facets.max_exclusive)
    {
        return Err(format!("{value} must be less than {text}"));
    }
    Ok(())
}
