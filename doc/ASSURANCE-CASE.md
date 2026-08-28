<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# Assurance case

An assurance case is an argument, supported by evidence, that the
software is adequately secure for what it does. This one is
deliberately short: the strongest security claim this project makes is
about what it *cannot* do.

## What this software is

`xmlschema` is an XSD 1.0 schema parser and validator.

## What it consumes

Its inputs are XML Schema documents and the XML instances validated against them — both untrusted. The threat model assumes every one of them is
hostile: a document written specifically to crash the parser, exhaust
memory, or reach something it should not.

## The claim

**A hostile input can cause this software to return an error. It
cannot cause it to corrupt memory, execute code, exhaust the machine,
or reach the network or the filesystem.**

## The argument

### Memory safety is structural, not tested for

Schema documents are parsed by `oxml`, which never fetches external entities, so a schema cannot reach the network or the filesystem.

### Resource exhaustion is bounded, not merely unlikely

Depth, entity expansion and input size are bounded by explicit limits
with documented defaults. Recursion is bounded because a stack
overflow aborts the process rather than unwinding, and no caller can
catch it.

### Correctness is measured against an external standard

The project does not grade its own homework. Where an independent
conformance suite exists it is run, its denominator is published
alongside its rate, and the result is ratcheted so an unreviewed change
in either direction fails the build.

## The evidence

- `#![forbid(unsafe_code)]`, checked by a CI job.
- The W3C XML Schema Test Suite (39,420 tests) runs in CI against a ratcheted baseline, currently 95.0% of 35,942 decided tests, and fails on movement in either direction.
- Two `cargo-fuzz` targets — `parse_schema` and `validate` — run on every pull request.
- 236 unit and integration tests; line coverage gated at a 95% floor.
- An unsupported construct is *skipped and recorded*, never silently treated as valid, so the published rate cannot flatter itself with accidental agreement.

## What this case does *not* claim

- It does not claim the absence of defects. It claims that a defect of
  a particular class — memory corruption — is ruled out by
  construction, and that other classes are bounded and tested for.
- It does not claim the defaults are the tightest possible. They are
  chosen to accept every real document encountered; a service parsing
  untrusted XML under load should tighten them.
- It does not claim independent review. This project has one
  maintainer, and no third party has audited it. That is recorded here
  rather than left to be inferred.

## Reporting a problem with this case

If you can construct an input that violates the claim above, that is a
vulnerability. See [SECURITY.md](../SECURITY.md).
