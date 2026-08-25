// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 xmlschema. All rights reserved.

//! Reading the suite's own catalogue.
//!
//! `suite.xml` references 32 `.testSet` files; each holds `testGroup`
//! elements, and each group holds one `schemaTest` — is this `.xsd` a
//! valid schema? — and zero or more `instanceTest`s — is this `.xml`
//! valid against it?
//!
//! Both kinds are counted. Loading only the instance tests would drop
//! 14,328 of the suite's 39,420 and quietly shrink the denominator.

use std::path::{Path, PathBuf};

use oxml::{Document, NodeId};

/// What a test expects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expected {
    /// The suite says this is valid.
    Valid,
    /// The suite says this is not.
    Invalid,
}

/// One test: a schema, optionally an instance, and an expectation.
#[derive(Debug, Clone)]
pub struct Case {
    /// The test group's name, as the suite gives it.
    pub group: String,
    /// The individual test's name.
    pub name: String,
    /// Which test set it came from, for per-submission reporting.
    pub set: String,
    /// The schema document, when the group names one.
    ///
    /// 55 groups carry an `instanceTest` with no `schemaTest`: their
    /// schema comes from a collection assembled through `xs:import`,
    /// which this harness cannot build. They are kept with `None` and
    /// run as blocked, because dropping them would shrink the
    /// denominator by 55 and report the shortfall as success.
    pub schema: Option<PathBuf>,
    /// The instance document, for an instance test.
    pub instance: Option<PathBuf>,
    /// What the suite expects.
    pub expected: Expected,
}

/// Load every test the suite defines.
///
/// # Errors
///
/// Returns a message if `suite.xml` cannot be read or parsed.
pub fn load(root: &Path) -> Result<Vec<Case>, String> {
    let suite = read(&root.join("suite.xml"))?;
    let doc = oxml::parse(&suite)
        .map_err(|e| format!("suite.xml is not well-formed: {e}"))?;

    let mut cases = Vec::new();
    for id in doc.descendants() {
        if local(&doc, id) != Some("testSetRef") {
            continue;
        }
        let Some(href) = doc.attribute(id, "href") else {
            continue;
        };
        let path = root.join(href);
        // A test set that fails to load is an error, not a skip: a
        // silently dropped set is a smaller denominator reported as
        // success.
        cases.extend(load_set(&path, root)?);
    }
    Ok(cases)
}

fn load_set(path: &Path, root: &Path) -> Result<Vec<Case>, String> {
    let text = read(path)?;
    let doc = oxml::parse(&text)
        .map_err(|e| format!("{} is not well-formed: {e}", path.display()))?;
    let base = path.parent().unwrap_or(root);

    let set = doc
        .root_element()
        .and_then(|r| doc.attribute(r, "name"))
        .unwrap_or("unknown")
        .to_owned();

    let mut cases = Vec::new();
    for group in doc.descendants() {
        if local(&doc, group) != Some("testGroup") {
            continue;
        }
        let name = doc.attribute(group, "name").unwrap_or("").to_owned();

        // The schema this group's instances validate against, which is
        // also a test in its own right. A group may have none.
        let schema_test = child_named(&doc, group, "schemaTest");
        let schema = schema_test
            .and_then(|t| child_named(&doc, t, "schemaDocument"))
            .and_then(|d| doc.attribute(d, "href"))
            .map(|href| normalise(base, href));

        if let (Some(test), Some(schema)) = (schema_test, schema.clone()) {
            if let Some(expected) = expectation(&doc, test) {
                cases.push(Case {
                    group: name.clone(),
                    name: doc
                        .attribute(test, "name")
                        .unwrap_or(&name)
                        .to_owned(),
                    set: set.clone(),
                    schema: Some(schema),
                    instance: None,
                    expected,
                });
            }
        }

        for &instance_test in doc.children(group) {
            if local(&doc, instance_test) != Some("instanceTest") {
                continue;
            }
            let Some(instance_doc) =
                child_named(&doc, instance_test, "instanceDocument")
            else {
                continue;
            };
            let Some(href) = doc.attribute(instance_doc, "href") else {
                continue;
            };
            let Some(expected) = expectation(&doc, instance_test) else {
                continue;
            };
            cases.push(Case {
                group: name.clone(),
                name: doc
                    .attribute(instance_test, "name")
                    .unwrap_or("instance")
                    .to_owned(),
                set: set.clone(),
                schema: schema.clone(),
                instance: Some(normalise(base, href)),
                expected,
            });
        }
    }
    Ok(cases)
}

/// A test's `expected` validity.
///
/// The suite sometimes carries more than one `expected`, qualified by
/// spec version. The unqualified one is the 1.0 expectation, which is
/// what this crate targets.
fn expectation(doc: &Document, test: NodeId) -> Option<Expected> {
    let mut fallback = None;
    for &child in doc.children(test) {
        if local(doc, child) != Some("expected") {
            continue;
        }
        let validity = match doc.attribute(child, "validity") {
            Some("valid") => Expected::Valid,
            Some("invalid" | "notKnown") => Expected::Invalid,
            _ => continue,
        };
        if doc.attribute(child, "version").is_none() {
            return Some(validity);
        }
        fallback.get_or_insert(validity);
    }
    fallback
}

/// Resolve an `xlink:href` against the file that contained it.
fn normalise(base: &Path, href: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    for part in href.split('/') {
        match part {
            "." | "" => {}
            ".." => {
                let _ = path.pop();
            }
            other => path.push(other),
        }
    }
    path
}

fn read(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))
}

fn local(doc: &Document, id: NodeId) -> Option<&str> {
    doc.element_name(id).map(|n| n.local.as_str())
}

fn child_named(doc: &Document, id: NodeId, name: &str) -> Option<NodeId> {
    doc.children(id)
        .iter()
        .copied()
        .find(|&c| local(doc, c) == Some(name))
}
