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

Untrusted input is checked at the boundaries documented in
[SECURITY.md](../SECURITY.md). This crate forbids unsafe Rust in its own
source. Those controls do not prove the absence of denial of service,
logic defects, or vulnerabilities in dependencies and runtimes.

## The argument

### Memory safety is structural, not tested for

Schema documents are parsed by `oxml`, which never fetches external entities, so a schema cannot reach the network or the filesystem.

### Resource exhaustion is bounded, not merely unlikely

The concrete source, recursion and evaluation limits, together with
caller-owned limits, are recorded in [the threat model](../SECURITY.md).
A stack overflow aborts the process, so recursion must be bounded before
entering another frame; aborting is not a mitigation. These controls do
not constitute a process-wide memory or concurrency bound.

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

- It does not claim the absence of defects or memory corruption in
  dependencies, foreign runtimes or the compiler. Source-level unsafe-code
  prohibition is one control, not a proof about the entire process.
- It does not claim that parser budgets replace deployment limits.
  Callers must separately bound ingress, concurrent requests, wall-clock
  execution and process or WebAssembly memory.
- It does not claim independent review. This project has one
  maintainer, and no third party has audited it. That is recorded here
  rather than left to be inferred.

## Reporting a problem with this case

If you can construct an input that violates the claim above, that is a
vulnerability. See [SECURITY.md](../SECURITY.md).
