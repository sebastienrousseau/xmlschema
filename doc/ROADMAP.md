<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# Roadmap

## Where this is

An XML Schema validator in pure Rust, built on `oxml`. Schema parsing,
structural validation, all 44 built-in simple types with their 12
restriction facets, complex types, derivation and diagnostics are
implemented and exercised against the W3C XSD test suite.

It is **not complete XSD 1.0**, and the README says so before it says
anything else. Against the 39,420-test W3C suite, 95.0% of the tests
that reach a decision pass. The gap between "decided" and "all" is the
honest measure of what is missing.

Validation is whole-document: the tree is parsed in full before any
check runs.

## The order

**1. Identity constraints** — `xs:key`, `xs:keyref`, `xs:unique`.
These are the largest remaining category of real schemas this cannot
validate, and unlike the import mechanisms they add validation power
rather than surface.

**2. Import mechanisms** — `xs:import`, `xs:include`, `xs:redefine`.
Deliberately after identity constraints: they multiply the surface
without deciding anything new, and a schema that spans files is not
more strictly checked than one that does not.

Note what happens today: those constructs are **skipped**, so
validation is incomplete rather than wrong. `support::unsupported()`
reports exactly which constructs a schema uses that this crate does
not enforce, so a caller can tell whether a "valid" verdict can be
taken at face value. That reporting is the reason the gap is safe to
have; it is not a substitute for closing it.

**3. Substitution groups**, and the undecidable corners of derivation
validity.

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
