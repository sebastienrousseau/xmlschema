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
/// Only the datatypes that carry a distinct *validation rule* are
/// modelled. `xs:token` and `xs:normalizedString`, for instance,
/// differ from `xs:string` in whitespace handling rather than in what
/// they accept, and are treated as strings here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltIn {
    /// `xs:string` and its whitespace-variant relatives.
    String,
    /// `xs:boolean` — `true`, `false`, `1`, `0`.
    Boolean,
    /// `xs:decimal`
    Decimal,
    /// `xs:integer`, `xs:int`, `xs:long`, `xs:short`.
    Integer,
    /// `xs:nonNegativeInteger`, `xs:positiveInteger` and friends.
    NonNegativeInteger,
    /// `xs:double` and `xs:float`.
    Double,
    /// `xs:date` — `YYYY-MM-DD`.
    Date,
    /// `xs:dateTime`
    DateTime,
    /// `xs:anyURI`
    AnyUri,
}

impl BuiltIn {
    /// Resolve an XSD type name, ignoring any namespace prefix.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        let local = name.rsplit(':').next().unwrap_or(name);
        Some(match local {
            "string" | "normalizedString" | "token" | "NMTOKEN" | "Name" | "NCName" | "ID"
            | "IDREF" | "language" => Self::String,
            "boolean" => Self::Boolean,
            "decimal" => Self::Decimal,
            "integer" | "int" | "long" | "short" | "byte" => Self::Integer,
            "nonNegativeInteger" | "positiveInteger" | "unsignedInt" | "unsignedLong"
            | "unsignedShort" => Self::NonNegativeInteger,
            "double" | "float" => Self::Double,
            "date" => Self::Date,
            "dateTime" => Self::DateTime,
            "anyURI" => Self::AnyUri,
            _ => return None,
        })
    }
}

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
}

impl Facets {
    /// Whether any facet is set.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// A simple type: a built-in, optionally restricted.
#[derive(Debug, Clone, PartialEq)]
pub struct SimpleType {
    /// The base built-in the value must first satisfy.
    pub base: BuiltIn,
    /// Additional constraints.
    pub facets: Facets,
}

/// What may appear inside an element.
#[derive(Debug, Clone, PartialEq)]
pub enum Content {
    /// No child elements; the text must satisfy the simple type.
    Simple(SimpleType),
    /// Children must appear in this order.
    Sequence(Vec<Particle>),
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
}

impl Schema {
    /// Look up a top-level element declaration.
    #[must_use]
    pub fn element(&self, name: &str) -> Option<&Particle> {
        self.elements.get(name)
    }
}
