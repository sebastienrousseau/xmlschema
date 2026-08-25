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
//! - Elements, `xs:sequence`, `xs:choice`, cardinality via
//!   `minOccurs` / `maxOccurs` including `unbounded`
//! - Attributes, with `use="required"`
//! - Built-in simple types: string, boolean, decimal, integer,
//!   non-negative integer, double, date, dateTime, anyURI
//! - Restriction facets: `enumeration`, `pattern`, `length`,
//!   `minLength`, `maxLength`, `minInclusive`, `maxInclusive`,
//!   `minExclusive`, `maxExclusive`
//! - Named top-level simple types, referenced by `type`
//!
//! ## Not yet supported
//!
//! `xs:all`, `xs:import` / `xs:include` / `xs:redefine`, substitution
//! groups, identity constraints (`xs:key`, `xs:keyref`, `xs:unique`),
//! type derivation by extension for complex content, and union or list
//! simple types. A schema using one of these is not rejected: the
//! construct is skipped and the surrounding rules still apply.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod datatype;
pub mod model;
pub mod parse;
pub mod pattern;
pub mod support;
pub mod validate;

pub use model::{
    AttributeDecl, BuiltIn, Content, Facets, NamespaceConstraint, Occurs,
    Particle, ProcessContents, Schema, SimpleType, Variety, Wildcard,
};
pub use parse::{SchemaError, parse_schema};
pub use pattern::{Pattern, PatternError};
pub use validate::{Report, Violation, validate};
