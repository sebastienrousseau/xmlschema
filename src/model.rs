// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 xmlschema. All rights reserved.

//! The schema model: what an `.xsd` says, in a form a validator can
//! walk.
//!
//! This is deliberately narrower than the XSD specification. It covers
//! the constructs that appear in real schemas — elements, sequences,
//! choices, cardinality, attributes, and the simple-type restriction
//! facets — and omits the ones that multiply surface without adding
//! validating power until the core is correct.

use std::collections::BTreeMap;

/// How many times a particle may occur.
///
/// `max` is `None` for `unbounded`. Keeping that as an `Option` rather
/// than a sentinel like `usize::MAX` means "unbounded" cannot be
/// confused with "a very large bound" in a comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Occurs {
    /// Minimum occurrences. `0` makes the particle optional.
    pub min: usize,
    /// Maximum occurrences, or `None` for `unbounded`.
    pub max: Option<usize>,
}

impl Default for Occurs {
    fn default() -> Self {
        Self {
            min: 1,
            max: Some(1),
        }
    }
}

impl Occurs {
    /// Whether `count` satisfies this cardinality.
    #[must_use]
    pub fn permits(self, count: usize) -> bool {
        count >= self.min && self.max.is_none_or(|m| count <= m)
    }

    /// A human-readable description, for diagnostics.
    #[must_use]
    pub fn describe(self) -> String {
        match (self.min, self.max) {
            (1, Some(1)) => "exactly once".to_owned(),
            (0, Some(1)) => "at most once".to_owned(),
            (0, None) => "any number of times".to_owned(),
            (n, None) => format!("at least {n} times"),
            (a, Some(b)) if a == b => format!("exactly {a} times"),
            (a, Some(b)) => format!("between {a} and {b} times"),
        }
    }
}

/// A built-in XSD simple type.
///
/// A built-in XSD datatype.
///
/// Re-exported from [`crate::datatype`], where every built-in is
/// modelled separately. It used to be a nine-variant summary that
/// folded `xs:byte` into an unbounded integer and `xs:NCName` into a
/// string; a schema using either then accepted values the
/// specification rejects, silently.
pub use crate::datatype::Datatype as BuiltIn;

/// Constraints narrowing a simple type.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Facets {
    /// `xs:enumeration` — the value must be one of these.
    pub enumeration: Vec<String>,
    /// `xs:pattern`, as written. See [`crate::pattern`] for the
    /// supported subset.
    pub pattern: Option<String>,
    /// `xs:minLength`
    pub min_length: Option<usize>,
    /// `xs:maxLength`
    pub max_length: Option<usize>,
    /// `xs:length`
    pub length: Option<usize>,
    /// `xs:minInclusive`
    pub min_inclusive: Option<f64>,
    /// `xs:maxInclusive`
    pub max_inclusive: Option<f64>,
    /// `xs:minExclusive`
    pub min_exclusive: Option<f64>,
    /// `xs:maxExclusive`
    pub max_exclusive: Option<f64>,
    /// `xs:totalDigits` — the count of significant digits.
    pub total_digits: Option<usize>,
    /// `xs:fractionDigits` — the count of digits after the point.
    pub fraction_digits: Option<usize>,
    /// `xs:whiteSpace` — overrides the base type's own rule.
    pub white_space: Option<crate::datatype::WhiteSpace>,
}

impl Facets {
    /// Whether any facet is set.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// A simple type: a built-in, optionally restricted, listed or united.
#[derive(Debug, Clone, PartialEq)]
pub struct SimpleType {
    /// The base built-in the value must first satisfy.
    ///
    /// Meaningful only for [`Variety::Atomic`]; a list or union
    /// carries its constraint in [`SimpleType::variety`] instead.
    pub base: BuiltIn,
    /// Additional constraints.
    pub facets: Facets,
    /// Whether the value is a single value, a whitespace-separated
    /// list of them, or one of several alternatives.
    pub variety: Variety,
}

impl SimpleType {
    /// An atomic type with no facets.
    #[must_use]
    pub fn atomic(base: BuiltIn) -> Self {
        Self {
            base,
            facets: Facets::default(),
            variety: Variety::Atomic,
        }
    }
}

/// Which of XSD's three simple-type varieties this is.
///
/// The specification calls these *varieties* rather than kinds, and
/// they are not interchangeable: length facets count characters on an
/// atomic type and *items* on a list, so folding a list into its item
/// type gets both the value space and the facets wrong.
#[derive(Debug, Clone, PartialEq)]
pub enum Variety {
    /// A single value.
    Atomic,
    /// A whitespace-separated sequence, every item of which satisfies
    /// the item type.
    List(Box<SimpleType>),
    /// A value satisfying at least one member type.
    Union(Vec<SimpleType>),
}

/// What may appear inside an element.
#[derive(Debug, Clone, PartialEq)]
pub enum Content {
    /// No child elements; the text must satisfy the simple type.
    ///
    /// Boxed because a `SimpleType` carries a `Variety`, which may
    /// hold a whole list item type or a vector of union members; left
    /// inline it makes every `Content` -- including the empty ones --
    /// as large as the largest simple type in the schema.
    Simple(Box<SimpleType>),
    /// Children must appear in this order.
    Sequence(Vec<Particle>),
    /// Children may appear in any order, each at most once.
    ///
    /// `xs:all` is not a `Sequence` with relaxed ordering: its
    /// particles are limited to `maxOccurs="1"`, and validating it as
    /// a sequence rejects a document whose children are simply in a
    /// different order.
    All(Vec<Particle>),
    /// Exactly one branch must match.
    Choice(Vec<Particle>),
    /// Any content is accepted. Used for `xs:any` and for element
    /// declarations with no type, which XSD treats as unconstrained.
    Any,
    /// No children and no text.
    Empty,
}

/// An element declaration within a content model.
#[derive(Debug, Clone, PartialEq)]
pub struct Particle {
    /// The element's local name.
    pub name: String,
    /// How many times it may occur.
    pub occurs: Occurs,
    /// What it may contain.
    pub content: Box<Content>,
    /// Its attribute declarations.
    pub attributes: Vec<AttributeDecl>,
    /// `@fixed` — the element's content must equal this exactly.
    pub fixed: Option<String>,
    /// `@nillable` — `xsi:nil="true"` may stand in for content.
    pub nillable: bool,
    /// When set, this particle is an `xs:any` wildcard rather than a
    /// named element, and [`Particle::name`] is empty.
    pub wildcard: Option<Wildcard>,
    /// `xs:anyAttribute` on this element's type, if it has one.
    pub any_attribute: Option<Wildcard>,
}

/// An `xs:any` or `xs:anyAttribute` wildcard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wildcard {
    /// Which namespaces it admits.
    pub namespaces: NamespaceConstraint,
    /// How strictly the matched content is validated.
    pub process: ProcessContents,
}

/// The `namespace` attribute of a wildcard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamespaceConstraint {
    /// `##any` — anything at all.
    Any,
    /// `##other` — anything outside the target namespace.
    Other,
    /// An explicit list, where `##targetNamespace` and `##local` have
    /// been resolved to a URI and to "no namespace" respectively.
    List(Vec<Option<String>>),
}

/// The `processContents` attribute of a wildcard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessContents {
    /// The matched element must have a declaration, and is validated
    /// against it.
    Strict,
    /// Validated if a declaration is found, accepted otherwise.
    Lax,
    /// Not validated at all.
    Skip,
}

/// An attribute declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct AttributeDecl {
    /// The attribute's local name.
    pub name: String,
    /// Whether it must be present.
    pub required: bool,
    /// The type its value must satisfy.
    pub simple_type: SimpleType,
    /// `@fixed` — if present, the value must equal this exactly.
    pub fixed: Option<String>,
    /// `@use="prohibited"` — the attribute must *not* appear.
    pub prohibited: bool,
}

/// A parsed schema.
#[derive(Debug, Clone)]
pub struct Schema {
    /// The schema's `targetNamespace`, if it declares one.
    pub target_namespace: Option<String>,
    /// Top-level element declarations, by local name.
    pub elements: BTreeMap<String, Particle>,
    /// Named top-level simple types, by local name.
    pub named_simple_types: BTreeMap<String, SimpleType>,
    /// Named top-level complex types, by local name.
    pub named_complex_types: BTreeMap<String, Content>,
}

impl Schema {
    /// Look up a top-level element declaration.
    #[must_use]
    pub fn element(&self, name: &str) -> Option<&Particle> {
        self.elements.get(name)
    }
}
