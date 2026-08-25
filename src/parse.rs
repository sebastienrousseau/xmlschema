// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 xmlschema. All rights reserved.

//! Reading an `.xsd` into a [`Schema`].

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;

use oxml::{Document, NodeId};

use crate::datatype::WhiteSpace;
use crate::model::{
    AttributeDecl, BuiltIn, Content, Facets, NamespaceConstraint, Occurs,
    Particle, ProcessContents, Schema, SimpleType, Variety, Wildcard,
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

/// Top-level declarations, by local name, as document nodes.
///
/// A `ref` names one of these, and so does a `group` or
/// `attributeGroup` reference. They are collected before anything is
/// resolved because a reference may point forward, and re-walking the
/// tree for each one is quadratic on a large schema.
#[derive(Debug, Default)]
struct Tops {
    elements: BTreeMap<String, NodeId>,
    attributes: BTreeMap<String, NodeId>,
    groups: BTreeMap<String, NodeId>,
    attribute_groups: BTreeMap<String, NodeId>,
    complex_types: BTreeMap<String, NodeId>,
}

/// Parsed complex types and attribute lists, by node.
///
/// Without this, resolving a named type re-parses it at every
/// reference, and each of *its* children resolves its own type in
/// turn: the work is exponential in the nesting depth. On the W3C
/// suite that was the difference between seconds and not finishing.
#[derive(Default)]
struct Memo {
    content: BTreeMap<usize, Content>,
    attributes: BTreeMap<usize, Vec<AttributeDecl>>,
}

/// The state one `parse_schema` call shares across its three passes.
///
/// Held together so a [`Ctx`] can be handed out without repeating six
/// fields at every call, and so the memo and the budget are plainly
/// per-parse rather than global.
struct Session<'a> {
    doc: &'a Document,
    tops: Tops,
    memo: RefCell<Memo>,
    budget: Cell<usize>,
}

impl Session<'_> {
    /// A context for parsing against the schema built so far.
    fn ctx<'s>(&'s self, schema: &'s Schema) -> Ctx<'s> {
        Ctx {
            doc: self.doc,
            schema,
            tops: &self.tops,
            memo: &self.memo,
            budget: &self.budget,
            depth: 0,
        }
    }
}

/// What a nested parse needs: the document, what has been built so
/// far, the memo, and how deep the reference chain is.
struct Ctx<'a> {
    doc: &'a Document,
    schema: &'a Schema,
    tops: &'a Tops,
    memo: &'a RefCell<Memo>,
    /// Guards against a group that references itself. XSD forbids a
    /// circular model group, but a schema is untrusted input and the
    /// alternative to a bound is a stack overflow.
    depth: usize,
    /// Particles built so far, against [`MAX_PARTICLES`].
    budget: &'a Cell<usize>,
}

impl<'a> Ctx<'a> {
    /// The same context one level deeper, or `None` at the limit.
    fn deeper(&self) -> Option<Ctx<'a>> {
        (self.depth < MAX_REFERENCE_DEPTH).then(|| Ctx {
            doc: self.doc,
            schema: self.schema,
            tops: self.tops,
            memo: self.memo,
            depth: self.depth + 1,
            budget: self.budget,
        })
    }

    /// Charge one particle against the budget.
    fn charge(&self) -> Result<(), SchemaError> {
        self.charge_many(1)
    }

    /// Charge `n` particles against the budget.
    ///
    /// A *use* of a type is charged, not a parse of it: the memo makes
    /// parsing linear, but each use clones the whole content model, so
    /// the cost that matters is the materialised tree.
    fn charge_many(&self, n: usize) -> Result<(), SchemaError> {
        let spent = self.budget.get().saturating_add(n);
        self.budget.set(spent);
        if spent > MAX_PARTICLES {
            return err(format!(
                "the schema's content models expand past {MAX_PARTICLES} \
                 particles; a type that references another twice doubles \
                 at every level"
            ));
        }
        Ok(())
    }
}

/// How far a chain of `ref`s and group references may nest.
const MAX_REFERENCE_DEPTH: usize = 64;

/// How many particles a schema's expanded content models may total.
///
/// A referenced type is *inlined* where it is used, so a schema whose
/// every type names the previous one twice doubles at each level: 24
/// levels is sixteen million particles, from a schema of a few
/// kilobytes. Depth alone does not bound that -- each level is within
/// the reference limit -- and neither does memoising the parse, since
/// the cost is in the materialised tree rather than the work to build
/// it.
///
/// Generous enough that no real schema approaches it: the largest in
/// the W3C suite expands to a few thousand.
const MAX_PARTICLES: usize = 100_000;

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
    let root = schema_root(&doc)?;
    check_structure(&doc)?;

    let session = Session {
        doc: &doc,
        tops: index_top_level(&doc, root),
        memo: RefCell::new(Memo::default()),
        budget: Cell::new(0),
    };

    let mut schema = Schema {
        target_namespace: doc
            .attribute(root, "targetNamespace")
            .map(str::to_owned),
        elements: BTreeMap::new(),
        named_simple_types: BTreeMap::new(),
        named_complex_types: BTreeMap::new(),
    };

    // Three passes, because a declaration may reference a type
    // declared after it and resolving forward references on demand
    // would re-walk the tree for each one.
    for &child in doc.children(root) {
        if local_name(&doc, child) == Some("simpleType") {
            if let Some(name) = doc.attribute(child, "name") {
                let st = parse_simple_type(&session.ctx(&schema), child);
                let _ = schema.named_simple_types.insert(name.to_owned(), st);
            }
        }
    }

    for (name, &node) in &session.tops.complex_types {
        let content = parse_complex_type(&session.ctx(&schema), node)?;
        let _ = schema.named_complex_types.insert(name.clone(), content);
    }

    for &child in doc.children(root) {
        if local_name(&doc, child) == Some("element") {
            let particle = parse_element(&session.ctx(&schema), child)?;
            let _ = schema.elements.insert(particle.name.clone(), particle);
        }
    }

    Ok(schema)
}

/// The `xs:schema` element, or why the document is not a schema.
fn schema_root(doc: &Document) -> Result<NodeId, SchemaError> {
    let Some(root) = doc.root_element() else {
        return err("the schema has no root element");
    };
    if local_name(doc, root) != Some("schema") {
        return err("the root element must be xs:schema");
    }
    Ok(root)
}

/// Index every top-level declaration by name.
///
/// Done before anything is resolved because a reference may point
/// forward, and re-walking the tree for each one is quadratic on a
/// large schema.
fn index_top_level(doc: &Document, root: NodeId) -> Tops {
    let mut tops = Tops::default();
    for &child in doc.children(root) {
        let (Some(kind), Some(name)) =
            (local_name(doc, child), doc.attribute(child, "name"))
        else {
            continue;
        };
        let table = match kind {
            "element" => &mut tops.elements,
            "attribute" => &mut tops.attributes,
            "group" => &mut tops.groups,
            "attributeGroup" => &mut tops.attribute_groups,
            "complexType" => &mut tops.complex_types,
            _ => continue,
        };
        let _ = table.insert(name.to_owned(), child);
    }
    tops
}

/// Check the schema document against XSD's own structural rules.
///
/// A schema is itself an XML document with a content model, and a
/// schema that breaks it is invalid however sensible its declarations
/// look. This crate reads what it recognises and ignored the rest,
/// so it accepted schemas the specification rejects -- around 180 in
/// the W3C suite before this.
///
/// Not the whole schema-for-schemas, which would be a second
/// validator. These are the structural rules the suite tests most:
/// where `annotation` may appear, and which children are mutually
/// exclusive.
fn check_structure(doc: &Document) -> Result<(), SchemaError> {
    for id in doc.descendants() {
        let Some(name) = local_name(doc, id) else {
            continue;
        };
        let children: Vec<&str> = doc
            .children(id)
            .iter()
            .filter_map(|&c| local_name(doc, c))
            .collect();

        // `annotation` is permitted once, and only first. Both halves
        // matter: `ctB002` repeats it, `ctB004` puts it last.
        let annotations =
            children.iter().filter(|c| **c == "annotation").count();
        if annotations > 1 {
            return err(format!(
                "xs:{name} has {annotations} xs:annotation children; \
                 at most one is permitted"
            ));
        }
        if annotations == 1 && children.first() != Some(&"annotation") {
            return err(format!(
                "xs:annotation must be the first child of xs:{name}"
            ));
        }

        // Mutually exclusive children, by parent.
        let exclusive: &[&str] = match name {
            "complexType" => &[
                "simpleContent",
                "complexContent",
                "group",
                "all",
                "choice",
                "sequence",
            ],
            "simpleType" => &["restriction", "list", "union"],
            "element" | "attribute" => &["simpleType", "complexType"],
            _ => &[],
        };
        let present: Vec<&&str> =
            children.iter().filter(|c| exclusive.contains(c)).collect();
        if present.len() > 1 {
            return err(format!(
                "xs:{name} has both xs:{} and xs:{}; they are mutually \
                 exclusive",
                present[0], present[1]
            ));
        }

        // A `simpleType` must say which variety it is.
        if name == "simpleType" && present.is_empty() {
            return err(
                "xs:simpleType must contain a restriction, list or union",
            );
        }

        // A declaration may name a type or contain one, not both.
        if matches!(name, "element" | "attribute")
            && doc.attribute(id, "type").is_some()
            && !present.is_empty()
        {
            return err(format!(
                "xs:{name} has both a `type` attribute and an inline type"
            ));
        }

        // A complexType with simpleContent or complexContent carries
        // everything inside it: attributes belong to the extension or
        // restriction, not beside the wrapper.
        if name == "complexType" {
            let wrapped = children
                .iter()
                .any(|c| matches!(*c, "simpleContent" | "complexContent"));
            if wrapped {
                if let Some(stray) = children.iter().find(|c| {
                    !matches!(
                        **c,
                        "simpleContent" | "complexContent" | "annotation"
                    )
                }) {
                    return err(format!(
                        "xs:{stray} may not sit beside xs:simpleContent or \
                         xs:complexContent; it belongs inside the extension \
                         or restriction"
                    ));
                }
            }
        }

        // Element Declarations Consistent: two elements of the same
        // name in one content model must have the same type.
        if matches!(name, "sequence" | "choice" | "all" | "group") {
            check_declarations_consistent(doc, id)?;
        }

        // `ref` excludes `name`, and everything a declaration would
        // carry.
        if matches!(name, "element" | "attribute")
            && doc.attribute(id, "ref").is_some()
            && doc.attribute(id, "name").is_some()
        {
            return err(format!("xs:{name} has both `name` and `ref`"));
        }
    }
    Ok(())
}

/// Two element declarations of the same name in one content model
/// must agree on their type.
///
/// XSD calls this *Element Declarations Consistent*. A model offering
/// `e1` as a string in one branch and as a complex type in another has
/// no single answer for what `e1` is, so the schema is invalid rather
/// than ambiguous.
///
/// The walk descends through nested model groups, because they are the
/// same content model, and stops at an element's own type, because
/// that is a different one.
fn check_declarations_consistent(
    doc: &Document,
    id: NodeId,
) -> Result<(), SchemaError> {
    let mut seen: Vec<(String, String)> = Vec::new();
    collect_declarations(doc, id, &mut seen);
    for (i, (name, signature)) in seen.iter().enumerate() {
        if let Some((_, other)) = seen[..i]
            .iter()
            .find(|(n, other)| n == name && other != signature)
        {
            return err(format!(
                "`{name}` is declared twice in one content model with \
                 different types (`{other}` and `{signature}`)"
            ));
        }
    }
    Ok(())
}

/// Element declarations directly within a content model, as
/// `(name, type signature)`.
fn collect_declarations(
    doc: &Document,
    id: NodeId,
    out: &mut Vec<(String, String)>,
) {
    for &child in doc.children(id) {
        match local_name(doc, child) {
            Some("element") => {
                // A `ref` names a top-level declaration, which is one
                // declaration however often it is referenced.
                if doc.attribute(child, "ref").is_some() {
                    continue;
                }
                let Some(name) = doc.attribute(child, "name") else {
                    continue;
                };
                // Only named types are compared. Two *inline* types
                // are separate components and a literal reading makes
                // them inconsistent, but the suite calls that shape
                // valid -- and a rule that wrongly rejects a valid
                // schema is worse than one that misses an invalid
                // one, so this stays narrow.
                let Some(signature) = doc.attribute(child, "type") else {
                    continue;
                };
                out.push((name.to_owned(), signature.to_owned()));
            }
            // A nested group is the same content model.
            Some("sequence" | "choice" | "all") => {
                collect_declarations(doc, child, out);
            }
            _ => {}
        }
    }
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

fn parse_element(ctx: &Ctx, id: NodeId) -> Result<Particle, SchemaError> {
    let doc = ctx.doc;

    // `<xs:element ref="name"/>` re-uses a top-level declaration, with
    // this occurrence's own cardinality. Resolving it is not optional:
    // an unresolved ref left the element unconstrained.
    if let Some(reference) = doc.attribute(id, "ref") {
        let local = reference.rsplit(':').next().unwrap_or(reference);
        // A reference this schema cannot resolve almost always names
        // something in an imported namespace, and `xs:import` is not
        // supported. Treating that as an invalid *schema* rejected 424
        // schemas the suite calls valid. It is not enforceable, which
        // `support::unsupported` reports; it is not wrong.
        let Some(&target) = ctx.tops.elements.get(local) else {
            return Ok(unenforceable_element(local, parse_occurs(doc, id)));
        };
        let Some(inner) = ctx.deeper() else {
            return Ok(unenforceable_element(local, parse_occurs(doc, id)));
        };
        let mut particle = parse_element(&inner, target)?;
        particle.occurs = parse_occurs(doc, id);
        return Ok(particle);
    }

    let Some(name) = doc.attribute(id, "name") else {
        return err("an xs:element has no name");
    };
    let occurs = parse_occurs(doc, id);

    // An element is typed one of three ways: a `type` attribute, an
    // inline complexType, or an inline simpleType. Anything else is
    // unconstrained.
    let content = if let Some(type_name) = doc.attribute(id, "type") {
        resolve_named_type(ctx, type_name)
    } else if let Some(ct) = first_child_named(doc, id, "complexType") {
        parse_complex_type(ctx, ct)?
    } else if let Some(st) = first_child_named(doc, id, "simpleType") {
        Content::Simple(Box::new(parse_simple_type(ctx, st)))
    } else {
        Content::Any
    };

    let attributes = if let Some(ct) = first_child_named(doc, id, "complexType")
    {
        parse_attributes(ctx, ct)?
    } else if let Some(type_name) = doc.attribute(id, "type") {
        // A named complex type carries attributes too.
        let local = type_name.rsplit(':').next().unwrap_or(type_name);
        match ctx.tops.complex_types.get(local) {
            Some(&node) => parse_attributes(ctx, node)?,
            None => Vec::new(),
        }
    } else {
        Vec::new()
    };

    ctx.charge()?;
    Ok(Particle {
        name: name.to_owned(),
        occurs,
        content: Box::new(content),
        attributes,
        fixed: doc.attribute(id, "fixed").map(str::to_owned),
        nillable: doc.attribute(id, "nillable") == Some("true"),
        wildcard: None,
        any_attribute: any_attribute_of(ctx, id),
    })
}

/// The `xs:anyAttribute` governing an element, from its inline
/// complexType or from the named one it uses.
fn any_attribute_of(ctx: &Ctx, id: NodeId) -> Option<Wildcard> {
    let doc = ctx.doc;
    let host = first_child_named(doc, id, "complexType").or_else(|| {
        let name = doc.attribute(id, "type")?;
        let local = name.rsplit(':').next().unwrap_or(name);
        ctx.tops.complex_types.get(local).copied()
    })?;
    // It may sit directly on the type or inside a derivation.
    let mut places = vec![host];
    for wrapper in ["complexContent", "simpleContent"] {
        if let Some(w) = first_child_named(doc, host, wrapper) {
            for kind in ["extension", "restriction"] {
                if let Some(node) = first_child_named(doc, w, kind) {
                    places.push(node);
                }
            }
        }
    }
    places
        .into_iter()
        .find_map(|p| first_child_named(doc, p, "anyAttribute"))
        .map(|node| parse_wildcard(ctx, node))
}

/// A particle for a reference that cannot be resolved here.
///
/// It keeps the name and cardinality so ordering still works, and
/// accepts any content, because this schema has nothing to check it
/// against. `support::unsupported` reports the import that caused it.
fn unenforceable_element(name: &str, occurs: Occurs) -> Particle {
    Particle {
        name: name.to_owned(),
        occurs,
        content: Box::new(Content::Any),
        attributes: Vec::new(),
        fixed: None,
        nillable: true,
        wildcard: None,
        any_attribute: None,
    }
}

fn resolve_named_type(ctx: &Ctx, name: &str) -> Content {
    let local = name.rsplit(':').next().unwrap_or(name);
    if let Some(st) = ctx.schema.named_simple_types.get(local) {
        return Content::Simple(Box::new(st.clone()));
    }
    if let Some(ct) = ctx.schema.named_complex_types.get(local) {
        return ct.clone();
    }
    // A complex type declared later in the same pass is not in the
    // schema yet, so fall back to reading it directly.
    if let Some(&node) = ctx.tops.complex_types.get(local) {
        if let Some(inner) = ctx.deeper() {
            if let Ok(content) = parse_complex_type(&inner, node) {
                return content;
            }
        }
    }
    BuiltIn::from_name(name).map_or(Content::Any, |b| {
        Content::Simple(Box::new(SimpleType::atomic(b)))
    })
}

fn parse_complex_type(ctx: &Ctx, id: NodeId) -> Result<Content, SchemaError> {
    let cached = ctx.memo.borrow().content.get(&id.index()).cloned();
    if let Some(hit) = cached {
        // Charged on every use, not only the first: this is another
        // copy of the whole model.
        ctx.charge_many(particle_count(&hit))?;
        return Ok(hit);
    }
    let content = parse_complex_type_uncached(ctx, id)?;
    ctx.charge_many(particle_count(&content))?;
    let _ = ctx
        .memo
        .borrow_mut()
        .content
        .insert(id.index(), content.clone());
    Ok(content)
}

/// How many particles a content model holds, counting nested ones.
fn particle_count(content: &Content) -> usize {
    match content {
        Content::Sequence(p) | Content::Choice(p) | Content::All(p) => p
            .iter()
            .map(|particle| 1 + particle_count(&particle.content))
            .sum(),
        _ => 0,
    }
}

fn parse_complex_type_uncached(
    ctx: &Ctx,
    id: NodeId,
) -> Result<Content, SchemaError> {
    let doc = ctx.doc;

    // `complexContent` wraps an extension or restriction of another
    // complex type. An extension appends its own particles to the
    // base's; a restriction replaces them.
    if let Some(cc) = first_child_named(doc, id, "complexContent") {
        return parse_complex_content(ctx, cc);
    }
    if let Some(group) = model_group(doc, id) {
        return parse_model_group(ctx, group);
    }
    if let Some(sc) = first_child_named(doc, id, "simpleContent") {
        // simpleContent restricts or extends a simple type; the
        // validating part is the base type.
        for kind in ["extension", "restriction"] {
            if let Some(node) = first_child_named(doc, sc, kind) {
                if let Some(base) = doc.attribute(node, "base") {
                    return Ok(resolve_named_type(ctx, base));
                }
            }
        }
        return Ok(Content::Any);
    }
    // A complexType with no particle and no simpleContent accepts
    // attributes only.
    Ok(Content::Empty)
}

/// Read an `xs:any` or `xs:anyAttribute`.
fn parse_wildcard(ctx: &Ctx, id: NodeId) -> Wildcard {
    let doc = ctx.doc;
    let target = ctx.schema.target_namespace.clone();
    let namespaces = match doc.attribute(id, "namespace") {
        None | Some("##any") => NamespaceConstraint::Any,
        Some("##other") => NamespaceConstraint::Other,
        Some(list) => NamespaceConstraint::List(
            list.split_whitespace()
                .map(|item| match item {
                    "##targetNamespace" => target.clone(),
                    "##local" => None,
                    uri => Some(uri.to_owned()),
                })
                .collect(),
        ),
    };
    let process = match doc.attribute(id, "processContents") {
        Some("skip") => ProcessContents::Skip,
        Some("lax") => ProcessContents::Lax,
        _ => ProcessContents::Strict,
    };
    Wildcard {
        namespaces,
        process,
    }
}

/// The `sequence`, `choice` or `all` directly inside `id`, if any.
fn model_group(doc: &Document, id: NodeId) -> Option<NodeId> {
    ["sequence", "choice", "all", "group"]
        .into_iter()
        .find_map(|name| first_child_named(doc, id, name))
}

/// Read a `sequence`, `choice`, `all`, or a reference to a named group.
fn parse_model_group(ctx: &Ctx, id: NodeId) -> Result<Content, SchemaError> {
    let doc = ctx.doc;
    match local_name(doc, id) {
        Some("sequence") => {
            Ok(Content::Sequence(group_particles(ctx, id, "sequence")?))
        }
        Some("choice") => {
            Ok(Content::Choice(group_particles(ctx, id, "choice")?))
        }
        Some("all") => Ok(Content::All(group_particles(ctx, id, "all")?)),
        Some("group") => {
            // A named group carries exactly one model group.
            let Some(reference) = doc.attribute(id, "ref") else {
                // A definition rather than a reference.
                return match model_group(doc, id) {
                    Some(inner) => parse_model_group(ctx, inner),
                    None => Ok(Content::Empty),
                };
            };
            let local = reference.rsplit(':').next().unwrap_or(reference);
            let Some(&target) = ctx.tops.groups.get(local) else {
                // As for an element reference: unresolvable means
                // unenforceable, not invalid.
                return Ok(Content::Any);
            };
            let Some(inner) = ctx.deeper() else {
                return Ok(Content::Any);
            };
            match model_group(doc, target) {
                Some(group) => parse_model_group(&inner, group),
                None => Ok(Content::Empty),
            }
        }
        _ => Ok(Content::Empty),
    }
}

/// `complexContent` — an extension or restriction of a complex type.
fn parse_complex_content(
    ctx: &Ctx,
    id: NodeId,
) -> Result<Content, SchemaError> {
    let doc = ctx.doc;
    let Some(node) = first_child_named(doc, id, "extension")
        .or_else(|| first_child_named(doc, id, "restriction"))
    else {
        return Ok(Content::Any);
    };
    let extending = local_name(doc, node) == Some("extension");

    let own = match model_group(doc, node) {
        Some(group) => parse_model_group(ctx, group)?,
        None => Content::Empty,
    };

    let Some(base_name) = doc.attribute(node, "base") else {
        return Ok(own);
    };
    let base = resolve_named_type(ctx, base_name);

    if !extending {
        // A restriction states the content model it permits in full.
        return Ok(own);
    }

    // An extension appends its particles to the base's, which is only
    // meaningful when both are sequences.
    Ok(match (base, own) {
        (Content::Sequence(mut a), Content::Sequence(b)) => {
            a.extend(b);
            Content::Sequence(a)
        }
        (Content::Empty, own) => own,
        // Anything else: the base decides. Appending a choice to a
        // sequence, say, has no single content model, and taking the
        // base is the reading that constrains rather than the one
        // that lets everything through.
        (base, _) => base,
    })
}

/// A model group's particles, with the group's own cardinality
/// applied.
///
/// `<xs:sequence maxOccurs="unbounded">` repeats the *group*, not the
/// element inside it. With one particle that is exactly the same as
/// multiplying its own cardinality, so it is done here. With more than
/// one it is not -- `(a, b){2}` permits `a b a b` and not `a a b b` --
/// and this crate has no repeated-group model, so the case is left to
/// `support::unsupported` to report rather than guessed at.
fn group_particles(
    ctx: &Ctx,
    id: NodeId,
    kind: &str,
) -> Result<Vec<Particle>, SchemaError> {
    let mut particles = parse_particles(ctx, id)?;
    let group = parse_occurs(ctx.doc, id);
    if group == Occurs::default() {
        return Ok(particles);
    }
    if let [only] = particles.as_mut_slice() {
        only.occurs = Occurs {
            min: only.occurs.min.saturating_mul(group.min),
            max: match (only.occurs.max, group.max) {
                (Some(a), Some(b)) => Some(a.saturating_mul(b)),
                // Either being unbounded makes the product unbounded.
                _ => None,
            },
        };
    }
    let _ = kind;
    Ok(particles)
}

fn parse_particles(
    ctx: &Ctx,
    id: NodeId,
) -> Result<Vec<Particle>, SchemaError> {
    let doc = ctx.doc;
    let mut out = Vec::new();
    for &child in doc.children(id) {
        match local_name(doc, child) {
            Some("element") => out.push(parse_element(ctx, child)?),
            Some("any") => out.push(Particle {
                name: String::new(),
                occurs: parse_occurs(doc, child),
                content: Box::new(Content::Any),
                attributes: Vec::new(),
                fixed: None,
                nillable: false,
                wildcard: Some(parse_wildcard(ctx, child)),
                any_attribute: None,
            }),
            // A nested model group or a group reference contributes its
            // own particles. Flattening loses the grouping, which
            // matters for a choice; that is recorded by
            // `support::unsupported` rather than silently ignored.
            Some("sequence" | "choice" | "all" | "group") => {
                let content = parse_model_group(ctx, child)?;
                match content {
                    Content::Sequence(p)
                    | Content::Choice(p)
                    | Content::All(p) => out.extend(p),
                    _ => {}
                }
            }
            _ => {}
        }
    }
    Ok(out)
}

/// Read a complexType's attribute declarations.
///
/// Follows `ref` to a top-level declaration and splices in any
/// `attributeGroup` the type references, both of which previously left
/// the attribute unconstrained.
fn parse_attributes(
    ctx: &Ctx,
    id: NodeId,
) -> Result<Vec<AttributeDecl>, SchemaError> {
    if let Some(hit) = ctx.memo.borrow().attributes.get(&id.index()) {
        return Ok(hit.clone());
    }
    let attributes = parse_attributes_uncached(ctx, id)?;
    let _ = ctx
        .memo
        .borrow_mut()
        .attributes
        .insert(id.index(), attributes.clone());
    Ok(attributes)
}

fn parse_attributes_uncached(
    ctx: &Ctx,
    id: NodeId,
) -> Result<Vec<AttributeDecl>, SchemaError> {
    let doc = ctx.doc;
    let mut out = Vec::new();

    // An extension or restriction may declare attributes of its own,
    // alongside the ones it inherits from its base.
    let mut hosts = vec![id];
    for wrapper in ["complexContent", "simpleContent"] {
        let Some(w) = first_child_named(doc, id, wrapper) else {
            continue;
        };
        for kind in ["extension", "restriction"] {
            let Some(node) = first_child_named(doc, w, kind) else {
                continue;
            };
            hosts.push(node);
            if let Some(base) = doc.attribute(node, "base") {
                let local = base.rsplit(':').next().unwrap_or(base);
                if let (Some(&target), Some(inner)) =
                    (ctx.tops.complex_types.get(local), ctx.deeper())
                {
                    out.extend(parse_attributes(&inner, target)?);
                }
            }
        }
    }

    for host in hosts {
        for &child in doc.children(host) {
            match local_name(doc, child) {
                Some("attribute") => {
                    if let Some(decl) = parse_attribute(ctx, child)? {
                        out.push(decl);
                    }
                }
                Some("attributeGroup") => {
                    let Some(reference) = doc.attribute(child, "ref") else {
                        continue;
                    };
                    let local =
                        reference.rsplit(':').next().unwrap_or(reference);
                    let Some(&target) = ctx.tops.attribute_groups.get(local)
                    else {
                        continue;
                    };
                    let Some(inner) = ctx.deeper() else {
                        continue;
                    };
                    out.extend(parse_attributes(&inner, target)?);
                }
                _ => {}
            }
        }
    }

    // A later declaration of the same name replaces an inherited one,
    // which is how a restriction narrows what it inherited.
    let mut seen: Vec<String> = Vec::new();
    out.reverse();
    out.retain(|d| {
        if seen.contains(&d.name) {
            false
        } else {
            seen.push(d.name.clone());
            true
        }
    });
    out.reverse();
    Ok(out)
}

/// One `xs:attribute`, following `ref` if it has one.
fn parse_attribute(
    ctx: &Ctx,
    id: NodeId,
) -> Result<Option<AttributeDecl>, SchemaError> {
    let doc = ctx.doc;
    let use_attr = doc.attribute(id, "use");

    if let Some(reference) = doc.attribute(id, "ref") {
        let local = reference.rsplit(':').next().unwrap_or(reference);
        let Some(&target) = ctx.tops.attributes.get(local) else {
            return Ok(None);
        };
        let Some(inner) = ctx.deeper() else {
            return Ok(None);
        };
        let Some(mut decl) = parse_attribute(&inner, target)? else {
            return Ok(None);
        };
        // This occurrence's own `use` and `fixed` win over the
        // declaration's.
        decl.required = use_attr == Some("required");
        decl.prohibited = use_attr == Some("prohibited");
        if let Some(fixed) = doc.attribute(id, "fixed") {
            decl.fixed = Some(fixed.to_owned());
        }
        return Ok(Some(decl));
    }

    let Some(name) = doc.attribute(id, "name") else {
        return Ok(None);
    };
    let simple_type = if let Some(t) = doc.attribute(id, "type") {
        match resolve_named_type(ctx, t) {
            Content::Simple(st) => *st,
            _ => SimpleType::atomic(BuiltIn::String),
        }
    } else if let Some(st) = first_child_named(doc, id, "simpleType") {
        parse_simple_type(ctx, st)
    } else {
        SimpleType::atomic(BuiltIn::String)
    };
    Ok(Some(AttributeDecl {
        name: name.to_owned(),
        required: use_attr == Some("required"),
        simple_type,
        fixed: doc.attribute(id, "fixed").map(str::to_owned),
        prohibited: use_attr == Some("prohibited"),
    }))
}

/// Read a `simpleType` into the model.
///
/// Infallible: a construct that cannot be resolved degrades to
/// `xs:string` rather than rejecting the whole schema. Callers that
/// need to know whether anything was skipped ask
/// [`crate::support::unsupported`], which audits the document rather
/// than trusting this to report.
fn parse_simple_type(ctx: &Ctx, id: NodeId) -> SimpleType {
    let doc = ctx.doc;
    // `list` and `union` are varieties in their own right, and are
    // checked before `restriction` because a restriction *of* a list
    // still has list-valued content.
    if let Some(list) = first_child_named(doc, id, "list") {
        return parse_list(ctx, list, Facets::default());
    }
    if let Some(union) = first_child_named(doc, id, "union") {
        return parse_union(ctx, union, Facets::default());
    }

    let Some(restriction) = first_child_named(doc, id, "restriction") else {
        return SimpleType::atomic(BuiltIn::String);
    };

    // A restriction whose base is a list or union keeps that variety
    // and adds facets to it; length facets then count *items*.
    let inherited = doc
        .attribute(restriction, "base")
        .and_then(|b| named_simple_type(b, ctx.schema))
        .filter(|st| st.variety != Variety::Atomic);

    let base_name = doc.attribute(restriction, "base").unwrap_or("string");
    let base = match resolve_named_type(ctx, base_name) {
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
            // A pattern that does not compile constrains nothing.
            // Dropping it here rather than failing keeps an
            // unsupported *regex* from being reported as an invalid
            // *document*; `support::unsupported` reports the schema.
            "pattern" => {
                facets.pattern = crate::pattern::Pattern::compile(value).ok();
            }
            "minLength" => facets.min_length = value.parse().ok(),
            "maxLength" => facets.max_length = value.parse().ok(),
            "length" => facets.length = value.parse().ok(),
            "minInclusive" => {
                facets.min_inclusive = Some(value.to_owned());
            }
            "maxInclusive" => {
                facets.max_inclusive = Some(value.to_owned());
            }
            "minExclusive" => {
                facets.min_exclusive = Some(value.to_owned());
            }
            "maxExclusive" => {
                facets.max_exclusive = Some(value.to_owned());
            }
            "totalDigits" => facets.total_digits = value.parse().ok(),
            "fractionDigits" => facets.fraction_digits = value.parse().ok(),
            "whiteSpace" => {
                facets.white_space = match value {
                    "preserve" => Some(WhiteSpace::Preserve),
                    "replace" => Some(WhiteSpace::Replace),
                    "collapse" => Some(WhiteSpace::Collapse),
                    _ => None,
                };
            }
            _ => {}
        }
    }
    if let Some(mut inherited) = inherited {
        inherited.facets = facets;
        return inherited;
    }
    // A restriction may nest the list or union inline, either
    // directly or wrapped in its own `simpleType`. The wrapped form
    // is what the specification's own examples use, and missing it
    // dropped the variety: a length facet then counted characters on
    // a list, which agrees with an item count on short values.
    for host in [
        Some(restriction),
        first_child_named(doc, restriction, "simpleType"),
    ]
    .into_iter()
    .flatten()
    {
        if let Some(list) = first_child_named(doc, host, "list") {
            return parse_list(ctx, list, facets);
        }
        if let Some(union) = first_child_named(doc, host, "union") {
            return parse_union(ctx, union, facets);
        }
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
fn parse_list(ctx: &Ctx, id: NodeId, facets: Facets) -> SimpleType {
    let doc = ctx.doc;
    let item = if let Some(name) = doc.attribute(id, "itemType") {
        item_type(ctx, name)
    } else if let Some(inline) = first_child_named(doc, id, "simpleType") {
        parse_simple_type(ctx, inline)
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
fn parse_union(ctx: &Ctx, id: NodeId, facets: Facets) -> SimpleType {
    let doc = ctx.doc;
    let mut members: Vec<SimpleType> = doc
        .attribute(id, "memberTypes")
        .unwrap_or_default()
        .split_whitespace()
        .map(|name| item_type(ctx, name))
        .collect();
    for &child in doc.children(id) {
        if local_name(doc, child) == Some("simpleType") {
            members.push(parse_simple_type(ctx, child));
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
fn item_type(ctx: &Ctx, name: &str) -> SimpleType {
    if let Some(named) = named_simple_type(name, ctx.schema) {
        return named;
    }
    BuiltIn::from_name(name)
        .map_or_else(|| SimpleType::atomic(BuiltIn::String), SimpleType::atomic)
}
