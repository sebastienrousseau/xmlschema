<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# Roadmap

## Where this is

An XML Schema validator in pure Rust, built on `oxml`. Schema parsing,
structural validation, all 44 built-in simple types with their 12
restriction facets, complex types, derivation and diagnostics are
implemented and exercised against the W3C XSD test suite.

It is **not complete XSD 1.0**, and the README says so before it says
anything else. Against the 39,420-test W3C suite, 96.1% of the tests
that reach a decision pass. The gap between "decided" and "all" is the
honest measure of what is missing.

Validation is whole-document: the tree is parsed in full before any
check runs.

## The order

**Implemented: identity constraints.** `xs:key`, `xs:keyref` and
`xs:unique` use scoped selectors and fields.

**Implemented: caller-supplied imports and includes.** `xs:import` and
`xs:include` resolve through `parse_schema_with` and a caller-owned
`SchemaSource`. The default resolver retrieves nothing.

**Remaining:** `xs:redefine`, substitution groups and unsupported
corners of derivation validity. `support::unsupported()` reports
constructs that are not enforced. A validation pass is not a complete
XSD verdict unless the caller also checks that report.

## What is deliberately absent

**XSD 1.1** — assertions, conditional type assignment. A different
specification, not a later part of this one. Xerces implements it.

**Streaming validation.** The document is parsed into a tree first.
Validating during a streaming parse would bound memory, but identity
constraints and derivation checks both need context the stream has
already passed, so it is not simply a smaller version of what exists.

**Repeated model groups.** `<xs:sequence maxOccurs="2">` repeats the
*group*: `(a, b){2}` permits `a b a b` and not `a a b b`. This crate
has no repeated-group model, and reports the construct through
`unsupported()` rather than enforcing the weaker constraint silently.

## Non-goals

Being a drop-in replacement for Xerces. This is a validator for
projects that want no C dependency and are willing to check the status
table against their schemas.
