# Security Policy

## Supported versions

xmlschema is pre-1.0. Only the most recent `0.0.x` release receives security
fixes.

## Reporting a vulnerability

Please report privately, not as a public issue:

- GitHub's [private vulnerability reporting](https://github.com/sebastienrousseau/xmlschema/security/advisories/new)
- or email <sebastian.rousseau@gmail.com>

You should get an acknowledgement within 72 hours.

## Threat model

Schemas and documents are untrusted input. The core parser supports
internal entities with configurable expansion/depth budgets and performs
no automatic external retrieval. `parse_schema` uses the core's default
profile, not its strict application profile. Callers must impose source,
time and process-memory limits appropriate to their workload.

Schema reference depth and expanded content-model particles are bounded.
`parse_schema_with` can resolve imports/includes only through the supplied
`SchemaSource`; that resolver owns any filesystem/network authorization.
`parse_schema` supplies no resolver. Callers must inspect
`support::unsupported` before treating a validation result as complete XSD
enforcement. A pass with unsupported constructs is not proof of validity.

**Memory safety.** This crate forbids unsafe Rust in its own source.
Dependencies and runtimes have separate safety obligations. This
does not rule out resource exhaustion, logic defects or protocol errors.

## What is not a vulnerability

- Rejecting a document other parsers accept, when the document is not
  well-formed. Please still report it — it is a bug, just not a
  security one.
