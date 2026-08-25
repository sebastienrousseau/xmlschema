// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 xmlschema. All rights reserved.

//! Reading an `.xsd` into a [`Schema`].

use std::collections::BTreeMap;

use oxml::{Document, NodeId};

use crate::model::{
    AttributeDecl, BuiltIn, Content, Facets, Occurs, Particle, Schema,
    SimpleType, Variety,
};

/// Why a schema could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaError {
    /// What is wrong.
    pub message: String,
}

impl std::fmt::Display for SchemaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for SchemaError {}

fn err<T>(message: impl Into<String>) -> Result<T, SchemaError> {
    Err(SchemaError {
        message: message.into(),
    })
}

/// Parse an XSD document into a [`Schema`].
///
/// # Errors
///
/// Returns [`SchemaError`] if the document is not a schema, or uses a
/// construct this implementation does not support.
pub fn parse_schema(xsd: &str) -> Result<Schema, SchemaError> {
    let doc = oxml::parse(xsd).map_err(|e| SchemaError {
        message: format!("the schema is not well-formed XML: {e}"),
    })?;
    let Some(root) = doc.root_element() else {
        return err("the schema has no root element");
    };
    if local_name(&doc, root) != Some("schema") {
        return err("the root element must be xs:schema");
    }

    let mut schema = Schema {
        target_namespace: doc
            .attribute(root, "targetNamespace")
            .map(str::to_owned),
        elements: BTreeMap::new(),
        named_simple_types: BTreeMap::new(),
    };

    // Named simple types are collected first: an element declaration
    // may reference one by name, and resolving forward references
    // afterwards would need a second pass over the whole tree.
    for &child in doc.children(root) {
        if local_name(&doc, child) == Some("simpleType") {
            if let Some(name) = doc.attribute(child, "name") {
                let st = parse_simple_type(&doc, child, &schema);
                let _ = schema.named_simple_types.insert(name.to_owned(), st);
            }
        }
    }

    for &child in doc.children(root) {
        if local_name(&doc, child) == Some("element") {
            let particle = parse_element(&doc, child, &schema)?;
            let _ = schema.elements.insert(particle.name.clone(), particle);
        }
    }

    if schema.elements.is_empty() {
        return err("the schema declares no top-level elements");
    }
    Ok(schema)
}

fn local_name(doc: &Document, id: NodeId) -> Option<&str> {
    doc.element_name(id).map(|n| n.local.as_str())
}

fn children_named<'a>(
    doc: &'a Document,
    id: NodeId,
    name: &'a str,
) -> impl Iterator<Item = NodeId> + 'a {
    doc.children(id)
        .iter()
        .copied()
        .filter(move |&c| local_name(doc, c) == Some(name))
}

fn first_child_named(doc: &Document, id: NodeId, name: &str) -> Option<NodeId> {
    children_named(doc, id, name).next()
}

fn parse_occurs(doc: &Document, id: NodeId) -> Occurs {
    let min = doc
        .attribute(id, "minOccurs")
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    let max = match doc.attribute(id, "maxOccurs") {
        Some("unbounded") => None,
        Some(v) => v.parse().ok().or(Some(1)),
        None => Some(1),
    };
    Occurs { min, max }
}

fn parse_element(
    doc: &Document,
    id: NodeId,
    schema: &Schema,
) -> Result<Particle, SchemaError> {
    let Some(name) = doc.attribute(id, "name") else {
        return err("an xs:element has no name");
    };
    let occurs = parse_occurs(doc, id);

    // An element is typed one of three ways: a `type` attribute, an
    // inline complexType, or an inline simpleType. Anything else is
    // unconstrained.
    let content = if let Some(type_name) = doc.attribute(id, "type") {
        resolve_named_type(type_name, schema)
    } else if let Some(ct) = first_child_named(doc, id, "complexType") {
        parse_complex_type(doc, ct, schema)?
    } else if let Some(st) = first_child_named(doc, id, "simpleType") {
        Content::Simple(parse_simple_type(doc, st, schema))
    } else {
        Content::Any
    };

    let attributes = if let Some(ct) = first_child_named(doc, id, "complexType")
    {
        parse_attributes(doc, ct, schema)
    } else {
        Vec::new()
    };

    Ok(Particle {
        name: name.to_owned(),
        occurs,
        content: Box::new(content),
        attributes,
    })
}

fn resolve_named_type(name: &str, schema: &Schema) -> Content {
    let local = name.rsplit(':').next().unwrap_or(name);
    if let Some(st) = schema.named_simple_types.get(local) {
        return Content::Simple(st.clone());
    }
    BuiltIn::from_name(name)
        .map_or(Content::Any, |b| Content::Simple(SimpleType::atomic(b)))
}

fn parse_complex_type(
    doc: &Document,
    id: NodeId,
    schema: &Schema,
) -> Result<Content, SchemaError> {
    if let Some(seq) = first_child_named(doc, id, "sequence") {
        return Ok(Content::Sequence(parse_particles(doc, seq, schema)?));
    }
    if let Some(choice) = first_child_named(doc, id, "choice") {
        return Ok(Content::Choice(parse_particles(doc, choice, schema)?));
    }
    if first_child_named(doc, id, "all").is_some() {
        return err(
            "xs:all is not supported yet; use xs:sequence or xs:choice",
        );
    }
    if let Some(sc) = first_child_named(doc, id, "simpleContent") {
        // simpleContent restricts or extends a simple type; the
        // validating part is the base type.
        if let Some(ext) = first_child_named(doc, sc, "extension") {
            if let Some(base) = doc.attribute(ext, "base") {
                return Ok(resolve_named_type(base, schema));
            }
        }
        return Ok(Content::Any);
    }
    // A complexType with no particle and no simpleContent accepts
    // attributes only.
    Ok(Content::Empty)
}

fn parse_particles(
    doc: &Document,
    id: NodeId,
    schema: &Schema,
) -> Result<Vec<Particle>, SchemaError> {
    let mut out = Vec::new();
    for child in children_named(doc, id, "element") {
        out.push(parse_element(doc, child, schema)?);
    }
    Ok(out)
}

/// Read a complexType's attribute declarations.
///
/// Infallible for the same reason as `parse_simple_type`: an
/// attribute without a name is skipped rather than rejected, and every
/// type resolution degrades to `xs:string`.
fn parse_attributes(
    doc: &Document,
    complex_type: NodeId,
    schema: &Schema,
) -> Vec<AttributeDecl> {
    let mut out = Vec::new();
    for attr in children_named(doc, complex_type, "attribute") {
        let Some(name) = doc.attribute(attr, "name") else {
            continue;
        };
        let required = doc.attribute(attr, "use") == Some("required");
        let simple_type = if let Some(t) = doc.attribute(attr, "type") {
            match resolve_named_type(t, schema) {
                Content::Simple(st) => st,
                _ => SimpleType::atomic(BuiltIn::String),
            }
        } else if let Some(st) = first_child_named(doc, attr, "simpleType") {
            parse_simple_type(doc, st, schema)
        } else {
            SimpleType::atomic(BuiltIn::String)
        };
        out.push(AttributeDecl {
            name: name.to_owned(),
            required,
            simple_type,
        });
    }
    out
}

/// Read a `simpleType` into the model.
///
/// Infallible: a construct that cannot be resolved degrades to
/// `xs:string` rather than rejecting the whole schema. Callers that
/// need to know whether anything was skipped ask
/// [`crate::support::unsupported`], which audits the document rather
/// than trusting this to report.
fn parse_simple_type(
    doc: &Document,
    id: NodeId,
    schema: &Schema,
) -> SimpleType {
    // `list` and `union` are varieties in their own right, and are
    // checked before `restriction` because a restriction *of* a list
    // still has list-valued content.
    if let Some(list) = first_child_named(doc, id, "list") {
        return parse_list(doc, list, schema, Facets::default());
    }
    if let Some(union) = first_child_named(doc, id, "union") {
        return parse_union(doc, union, schema, Facets::default());
    }

    let Some(restriction) = first_child_named(doc, id, "restriction") else {
        return SimpleType::atomic(BuiltIn::String);
    };

    // A restriction whose base is a list or union keeps that variety
    // and adds facets to it; length facets then count *items*.
    let inherited = doc
        .attribute(restriction, "base")
        .and_then(|b| named_simple_type(b, schema))
        .filter(|st| st.variety != Variety::Atomic);

    let base_name = doc.attribute(restriction, "base").unwrap_or("string");
    let base = match resolve_named_type(base_name, schema) {
        Content::Simple(st) => st.base,
        _ => BuiltIn::String,
    };

    let mut facets = Facets::default();
    for &facet in doc.children(restriction) {
        let Some(kind) = local_name(doc, facet) else {
            continue;
        };
        let Some(value) = doc.attribute(facet, "value") else {
            continue;
        };
        match kind {
            "enumeration" => facets.enumeration.push(value.to_owned()),
            "pattern" => facets.pattern = Some(value.to_owned()),
            "minLength" => facets.min_length = value.parse().ok(),
            "maxLength" => facets.max_length = value.parse().ok(),
            "length" => facets.length = value.parse().ok(),
            "minInclusive" => facets.min_inclusive = value.parse().ok(),
            "maxInclusive" => facets.max_inclusive = value.parse().ok(),
            "minExclusive" => facets.min_exclusive = value.parse().ok(),
            "maxExclusive" => facets.max_exclusive = value.parse().ok(),
            _ => {}
        }
    }
    if let Some(mut inherited) = inherited {
        inherited.facets = facets;
        return inherited;
    }
    // A restriction may also nest the list or union inline.
    if let Some(list) = first_child_named(doc, restriction, "list") {
        return parse_list(doc, list, schema, facets);
    }
    if let Some(union) = first_child_named(doc, restriction, "union") {
        return parse_union(doc, union, schema, facets);
    }
    SimpleType {
        base,
        facets,
        variety: Variety::Atomic,
    }
}

/// A named top-level simple type, if the schema declares one.
fn named_simple_type(name: &str, schema: &Schema) -> Option<SimpleType> {
    let local = name.rsplit(':').next().unwrap_or(name);
    schema.named_simple_types.get(local).cloned()
}

/// `<xs:list itemType="..."/>` or `<xs:list><xs:simpleType>…`.
fn parse_list(
    doc: &Document,
    id: NodeId,
    schema: &Schema,
    facets: Facets,
) -> SimpleType {
    let item = if let Some(name) = doc.attribute(id, "itemType") {
        item_type(name, schema)
    } else if let Some(inline) = first_child_named(doc, id, "simpleType") {
        parse_simple_type(doc, inline, schema)
    } else {
        SimpleType::atomic(BuiltIn::String)
    };
    SimpleType {
        base: BuiltIn::AnySimpleType,
        facets,
        variety: Variety::List(Box::new(item)),
    }
}

/// `<xs:union memberTypes="a b"/>`, with any nested `simpleType`s
/// added to the named ones.
fn parse_union(
    doc: &Document,
    id: NodeId,
    schema: &Schema,
    facets: Facets,
) -> SimpleType {
    let mut members: Vec<SimpleType> = doc
        .attribute(id, "memberTypes")
        .unwrap_or_default()
        .split_whitespace()
        .map(|name| item_type(name, schema))
        .collect();
    for &child in doc.children(id) {
        if local_name(doc, child) == Some("simpleType") {
            members.push(parse_simple_type(doc, child, schema));
        }
    }
    if members.is_empty() {
        members.push(SimpleType::atomic(BuiltIn::String));
    }
    SimpleType {
        base: BuiltIn::AnySimpleType,
        facets,
        variety: Variety::Union(members),
    }
}

/// Resolve a type name used as a list item or union member.
fn item_type(name: &str, schema: &Schema) -> SimpleType {
    if let Some(named) = named_simple_type(name, schema) {
        return named;
    }
    BuiltIn::from_name(name)
        .map_or_else(|| SimpleType::atomic(BuiltIn::String), SimpleType::atomic)
}
