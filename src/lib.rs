// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 xmlschema. All rights reserved.

//! # xmlschema
//!
//! XML Schema (XSD) validation for Rust. Pure safe code, no C.
//!
//! The schema member of the [oxml](https://github.com/sebastienrousseau/oxml)
//! suite. Rust has had no pure-Rust XSD validator: the only option was
//! `libxml`, which binds libxml2 through C-FFI and so brings a build
//! toolchain, `unsafe`, and no WebAssembly support. This closes that
//! gap.
//!
//! ## Quick Start
//!
//! ```
//! use xmlschema::{parse_schema, validate};
//!
//! let xsd = r#"
//!   <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
//!     <xs:element name="book">
//!       <xs:complexType>
//!         <xs:sequence>
//!           <xs:element name="title" type="xs:string"/>
//!           <xs:element name="year" type="xs:integer"/>
//!         </xs:sequence>
//!         <xs:attribute name="lang" type="xs:string" use="required"/>
//!       </xs:complexType>
//!     </xs:element>
//!   </xs:schema>
//! "#;
//!
//! let schema = parse_schema(xsd)?;
//! let doc = oxml::parse(
//!     "<book lang='en'><title>Dune</title><year>1965</year></book>"
//! )?;
//!
//! assert!(validate(&doc, &schema).is_valid());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Every violation, not the first
//!
//! Validation collects rather than short-circuits, and each violation
//! carries the path to the element it is about. A caller fixing a
//! document wants the whole list, not to re-run the validator after
//! every edit:
//!
//! ```
//! use xmlschema::{parse_schema, validate};
//!
//! let xsd = r#"
//!   <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
//!     <xs:element name="book">
//!       <xs:complexType>
//!         <xs:sequence>
//!           <xs:element name="year" type="xs:integer"/>
//!         </xs:sequence>
//!         <xs:attribute name="lang" type="xs:string" use="required"/>
//!       </xs:complexType>
//!     </xs:element>
//!   </xs:schema>
//! "#;
//!
//! let schema = parse_schema(xsd)?;
//! let doc = oxml::parse("<book><year>not-a-number</year></book>")?;
//! let report = validate(&doc, &schema);
//!
//! // Both problems, in one pass.
//! assert_eq!(report.violations.len(), 2);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Supported
//!
//! - Elements, `xs:sequence`, `xs:choice`, `xs:all`, cardinality via
//!   `minOccurs` / `maxOccurs` including `unbounded`, `nillable`, `fixed`
//! - Reusable `xs:group` and `xs:attributeGroup` definitions (`ref`)
//! - Attributes, with `use="required"`, `use="prohibited"`, and `fixed`
//! - Wildcards: `xs:any` and `xs:anyAttribute` with `namespace` and
//!   `processContents` (`strict`, `lax`, `skip`)
//! - All 44 XSD 1.0 built-in simple types (strings, numbers, dates,
//!   durations, binary, names, URIs, `QNames`)
//! - Simple-type varieties: atomic restrictions, `xs:list`, and `xs:union`
//! - All 12 restriction facets: `enumeration`, `pattern`, `length`,
//!   `minLength`, `maxLength`, `minInclusive`, `maxInclusive`,
//!   `minExclusive`, `maxExclusive`, `totalDigits`, `fractionDigits`,
//!   `whiteSpace`
//! - Named top-level simple and complex types, with `simpleContent` and
//!   `complexContent` derivation by `extension` and `restriction`
//! - Identity constraints: `xs:unique`, `xs:key`, and `xs:keyref`
//!   (`xs:selector` and `xs:field`)
//! - Multi-file schemas (`xs:import` and `xs:include`) via caller-supplied
//!   [`SchemaSource`]
//!
//! ## Not yet supported
//!
//! `xs:redefine`, substitution groups, `@abstract`, `@block`, `@final`,
//! `@default` value injection, `@mixed` content enforcement, and the full
//! Schema Component Constraint (UPA / particle attribution) state machine.
//! Use [`support::unsupported`] to inspect a schema for constructs this
//! crate does not enforce.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod datatype;
pub mod derive;
pub mod model;
pub mod parse;
pub mod pattern;
pub mod resolve;
pub mod support;
pub mod validate;

pub use model::{
    AttributeDecl, BuiltIn, Content, Facets, Identity, IdentityKind,
    NamespaceConstraint, Occurs, Particle, ProcessContents, Schema, SimpleType,
    Variety, Wildcard,
};
pub use parse::{SchemaError, parse_schema, parse_schema_with};
pub use pattern::{Pattern, PatternError};
pub use resolve::{NoSchemas, SchemaSource};
pub use validate::{Report, Violation, validate};
