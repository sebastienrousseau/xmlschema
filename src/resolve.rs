// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 xmlschema. All rights reserved.

//! Schema documents the caller supplies, for `xs:import` and
//! `xs:include`.
//!
//! Neither this crate nor `oxml` performs I/O. A schema referencing
//! another names a *location*, and resolving it is the caller's
//! decision -- they have the permission model, the user, and the
//! context to make it. A schema processor that fetches by default is
//! how a validator becomes an outbound request.
//!
//! So [`parse_schema`] resolves nothing and reports every `xs:import`
//! and `xs:include` as unenforceable. [`parse_schema_with`] takes a
//! [`SchemaSource`] and asks it, by `schemaLocation`, for the
//! documents it needs.
//!
//! [`parse_schema`]: crate::parse_schema
//! [`parse_schema_with`]: crate::parse_schema_with

/// Somewhere the caller can look up a schema document.
///
/// Implemented for `&[(&str, &str)]`, which is enough for a test
/// fixture or a set of schemas already in memory.
///
/// # Examples
///
/// ```
/// use xmlschema::{SchemaSource, parse_schema_with, validate};
///
/// let common = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
///     <xs:simpleType name="code">
///       <xs:restriction base="xs:string">
///         <xs:maxLength value="4"/>
///       </xs:restriction>
///     </xs:simpleType>
///   </xs:schema>"#;
///
/// let main = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
///     <xs:include schemaLocation="common.xsd"/>
///     <xs:element name="r" type="code"/>
///   </xs:schema>"#;
///
/// let parts: &[(&str, &str)] = &[("common.xsd", common)];
/// assert_eq!(parts.fetch("common.xsd"), Some(common));
///
/// let schema = parse_schema_with(main, &parts)?;
/// let doc = oxml::parse("<r>abcd</r>")?;
/// assert!(validate(&doc, &schema).is_valid());
///
/// // The included type is enforced, which is the whole point.
/// let too_long = oxml::parse("<r>abcde</r>")?;
/// assert!(!validate(&too_long, &schema).is_valid());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub trait SchemaSource {
    /// The document at a `schemaLocation`, or `None` if unavailable.
    ///
    /// Returning `None` is not an error: it means the same as having
    /// no source for that location, so a caller can supply the parts
    /// they have and leave the rest. What is not supplied is reported
    /// as unenforceable rather than silently ignored.
    fn fetch(&self, location: &str) -> Option<&str>;
}

impl SchemaSource for [(&str, &str)] {
    fn fetch(&self, location: &str) -> Option<&str> {
        self.iter()
            .find(|(at, _)| *at == location)
            .map(|(_, text)| *text)
    }
}

impl<T: SchemaSource + ?Sized> SchemaSource for &T {
    fn fetch(&self, location: &str) -> Option<&str> {
        (**self).fetch(location)
    }
}

/// A source that supplies nothing, which is what [`parse_schema`] uses.
///
/// [`parse_schema`]: crate::parse_schema
#[derive(Debug, Clone, Copy)]
pub struct NoSchemas;

impl SchemaSource for NoSchemas {
    fn fetch(&self, _: &str) -> Option<&str> {
        None
    }
}
