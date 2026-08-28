<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

<h1 align="center">xmlschema</h1>

<p align="center">
  XML Schema (XSD) validation for Rust — the schema member of the
  <a href="https://github.com/sebastienrousseau/oxml">oxml</a> suite,
  with zero <code>unsafe</code> code.
</p>

<p align="center">
  <a href="https://github.com/sebastienrousseau/xmlschema/actions"><img src="https://img.shields.io/github/actions/workflow/status/sebastienrousseau/xmlschema/ci.yml?style=for-the-badge&logo=github" alt="Build" /></a>
  <a href="https://crates.io/crates/xmlschema"><img src="https://img.shields.io/crates/v/xmlschema.svg?style=for-the-badge&color=fc8d62&logo=rust" alt="Crates.io" /></a>
  <a href="https://docs.rs/xmlschema"><img src="https://img.shields.io/badge/docs.rs-xmlschema-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs" alt="Docs.rs" /></a>
  <a href="https://lib.rs/crates/xmlschema"><img src="https://img.shields.io/badge/lib.rs-xmlschema-orange.svg?style=for-the-badge" alt="lib.rs" /></a>
  <a href="https://scorecard.dev/viewer/?uri=github.com/sebastienrousseau/xmlschema"><img src="https://img.shields.io/ossf-scorecard/github.com/sebastienrousseau/xmlschema?style=for-the-badge&label=OpenSSF%20Scorecard&logo=openssf" alt="OpenSSF Scorecard" /></a>
</p>

---

> [!NOTE]
> **The rewrite has landed.** Schema parsing, structural validation,
> simple types with restriction facets, and `xs:pattern` all work; see
> [Status](#status) for exactly what is and is not supported.
>
> `0.0.1` on crates.io is the *old* crate, which exposed no public API
> at all. Do not use it.

## Contents

**Getting started**

- [Status](#status) — what works today, honestly
- [Install](#install) — Cargo
- [Quick Start](#quick-start) — validate a document in ten lines

**The oxml ecosystem**

- [The oxml ecosystem](#the-oxml-ecosystem) — six crates, one version

**Reference**

- [Why this crate exists](#why-this-crate-exists) — the gap it fills
- [Ecosystem comparison](#ecosystem-comparison) — XSD support in Rust
- [Reading a report](#reading-a-report) — every violation, each with a path
- [Migration](#migration) — from `libxml`
- [What is not implemented](#what-is-not-implemented) — and what comes next
- [Benchmarks](#benchmarks) — schema parsing, validation, the pattern engine

**Practical**

- [Examples](#examples) — runnable, and run in CI
- [When not to use xmlschema](#when-not-to-use-xmlschema)
- [FAQ](#faq)
- [Development](#development)
- [Security](#security)
- [Documentation](#documentation)
- [Acknowledgements](#acknowledgements)
- [License](#license)

---

## Status

| | State |
|---|---|
| Schema parsing | ✅ elements, model groups, cardinality, attributes |
| Simple types | ✅ all 44 built-ins, 12 restriction facets |
| `xs:list` and `xs:union` | ✅ |
| `xs:all`, `xs:group`, `xs:attributeGroup` | ✅ |
| `xs:any` and `xs:anyAttribute` | ✅ namespace and `processContents` |
| Complex-type derivation | ✅ extension, restriction, and *Particle Valid (Restriction)* |
| Schema validity | ✅ XSD's own structural rules |
| `xs:pattern` | ✅ own engine, XSD dialect including class subtraction |
| Diagnostics | ✅ every violation, each with a path |
| Conformance | ✅ **95.0%** of the W3C suite's decided tests, ratcheted |
| Tests | ✅ 236, plus the conformance suite |
| Identity constraints (`key`, `keyref`, `unique`) | ✗ |
| `xs:import` / `include` | ✗ |
| Substitution groups | ✗ |

An unsupported construct is skipped rather than rejected: the
surrounding rules still apply, so a schema using one validates
everything else correctly instead of failing wholesale.

**What was skipped is reported.** `support::unsupported` audits a
schema against what this crate enforces and names everything it does
not, so "this document is valid" and "this document was checked" are
never confused for one another.

## Install

```toml
[dependencies]
xmlschema = { git = "https://github.com/sebastienrousseau/xmlschema" }
oxml = { git = "https://github.com/sebastienrousseau/oxml" }
```

Published releases follow once the suite cuts its first version
together.

## Quick Start

```rust
use xmlschema::{parse_schema, validate};

let xsd = r#"
  <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
    <xs:element name="book">
      <xs:complexType>
        <xs:sequence>
          <xs:element name="title" type="xs:string"/>
        </xs:sequence>
        <xs:attribute name="lang" type="xs:string" use="required"/>
      </xs:complexType>
    </xs:element>
  </xs:schema>
"#;

let schema = parse_schema(xsd)?;
let doc = oxml::parse("<book lang='en'><title>Dune</title></book>")?;

assert!(validate(&doc, &schema).is_valid());
# Ok::<(), Box<dyn std::error::Error>>(())
```

Every violation is reported, each with a path:

```text
/invoice/issued — `22/08/2026` is not a valid date (YYYY-MM-DD)
/invoice/line[1]/@currency — `pounds` does not match the pattern `[A-Z]{3}`
/invoice/line[1]/amount — -5 must be greater than 0
/invoice/line[2] — missing required attribute `currency`
/invoice/line[2]/amount — `not a number` is not a valid decimal
```

## Why this crate exists

Rust has no pure-Rust XSD validator. The options today are:

- **[`libxml`](https://crates.io/crates/libxml)** — bindings to
  libxml2. Complete and battle-tested, but it is C: it needs a build
  toolchain, contains `unsafe`, does not work in WebAssembly, and
  inherits libxml2's CVE stream.
- **Nothing else.** There is no maintained pure-Rust implementation.

For a project already committed to safe Rust — no C toolchain, WASM
targets, an auditable dependency tree — that is not a choice so much as
an absence.

`xmlschema` exists to close it, with the same constraints as the rest
of the suite: `#![forbid(unsafe_code)]`, no FFI, no C.

## The oxml ecosystem

Every member ships the **same version number**, so there is never a
compatibility table to consult.

| Crate | What it is | Status |
|---|---|---|
| [`oxml`](https://github.com/sebastienrousseau/oxml) | Core — parser, tree, XPath 1.0 | **Available** |
| [`oxml-cli`](https://github.com/sebastienrousseau/oxml-cli) | Command-line querying and formatting | **Available** |
| [`oxml-lsp`](https://github.com/sebastienrousseau/oxml-lsp) | XML analysis and linting; the LSP transport is not yet implemented | **Available** |
| [`oxml-mcp`](https://github.com/sebastienrousseau/oxml-mcp) | Model Context Protocol server | **Available** |
| [`oxml-wasm`](https://github.com/sebastienrousseau/oxml-wasm) | WebAssembly bindings | **Available** |
| **`xmlschema`** | **XSD validation** | **Being rewritten** |

This crate keeps its published name rather than being folded into
`oxml`. The name means XSD validation specifically, and repurposing it
into a general toolkit would have handed existing users something
entirely different under a name they already depend on.

## Ecosystem comparison

| Crate | XSD validation | Pure Rust | WASM | Status |
|---|---|---|---|---|
| **`xmlschema`** | ✅ 95.0% of the W3C suite's decided tests | ✅ | ✅ | active |
| `libxml` | ✅ | ✗ (C-FFI) | ✗ | active |
| `quick-xml` | ✗ | ✅ | ✅ | active |
| `roxmltree` | ✗ | ✅ | ✅ | active |
| `xot` | ✗ | ✅ | ✅ | active |

`libxml` remains the more complete implementation. The difference is
what it costs: a C toolchain, `unsafe`, no WebAssembly target, and
libxml2's CVE stream. This crate trades completeness for those.

## What is not implemented

Everything in the list this section used to hold has shipped — schema
parsing, structural validation, simple types, complex types and
diagnostics are all ✅ in [Status](#status) above. What remains:

1. **Identity constraints** — `xs:key`, `xs:keyref`, `xs:unique`.
2. **Import mechanisms** — `xs:import`, `xs:include`, `xs:redefine`.
   These come after the core is correct, because they multiply the
   surface without adding validation power.
3. **Substitution groups**, and the undecidable corners of derivation
   validity.

An unsupported construct is skipped rather than rejected, and the
conformance harness counts such a test as *unsupported* whatever
answer it produced — so the published rate never flatters itself with
accidental agreement.

## Benchmarks

```bash
cargo bench --bench schema     # parsing an .xsd into the model
cargo bench --bench validate   # validating documents against it
cargo bench --bench pattern    # the `xs:pattern` engine
```

Three benchmarks, split because they answer different questions.
Parsing a schema is the expensive half and happens once; validating is
the cheap half and repeats. The pattern engine is separate because it
is a regex implementation of its own, in XSD's dialect rather than
PCRE's.

No absolute figures are published here. The same benchmarks on this
machine returned confidence intervals spanning 566–906 µs for a single
case — a spread wider than most changes worth measuring — because the
machine was busy. A figure without its conditions is not a
measurement. See
[oxml's BENCHMARKS.md](https://github.com/sebastienrousseau/oxml/blob/main/doc/BENCHMARKS.md)
for the method and what a published number has to carry.

## Reading a report

`validate` returns a `Report`, not a `Result`. A document can be wrong
in several independent ways, and stopping at the first means fixing
them one build at a time.

```text
/order: missing required attribute `id`
/order: expected `customer` exactly once, found 0
/order/line[1]: expected `sku` exactly once, found 0
/order/line[1]/qty: `many` is not a valid integer
/order/line[1]/sku: unexpected element `sku`; this content model allows sku, qty in that order
```

Each `Violation` carries a `path` and a `message`. The path is
positional — `line[1]` is the first `line` child — so it identifies one
element rather than a set.

## Migration

### From `xmllint --schema`

| `xmllint` | `xmlschema` |
|---|---|
| `xmllint --schema s.xsd --noout f.xml` | `validate(&parse(xml)?, &parse_schema(xsd)?)` |
| exit status | `report.is_valid()` |
| stderr text | `report.violations`, each with a path |
| `--schema` with `xs:import` | not supported yet |

The useful difference is that violations are data rather than a stream
of text to grep.

### From `libxml`'s `XmlSchemaValidationContext`

| `libxml` | `xmlschema` |
|---|---|
| `SchemaParserContext::from_buffer` | `parse_schema` |
| `SchemaValidationContext::validate_document` | `validate` |
| error callbacks | `report.violations` |
| a libxml2 C dependency | none |

`libxml2` implements XSD 1.0 completely and this crate does not — see
[Status](#status). If you need `xs:import`, identity constraints or
complex-type derivation today, stay.

## Examples

[`examples/`](examples/) is compiled and run in CI.

| Example | What it shows |
|---|---|
| [`validate`](examples/validate.rs) | Parsing a schema once, validating many, and reading a `Report` |

```bash
cargo run --example validate
```

## When not to use xmlschema

- **You need complete XSD 1.0.** This is early; check
  [Status](#status) against your schemas first.
- **You need XSD 1.1** — assertions, conditional type assignment.
  Xerces has it.
- **Your schemas use `xs:import` or `xs:include`.** Not supported;
  those constructs are skipped, so validation is incomplete rather
  than wrong.
- **You need identity constraints** — `xs:key`, `xs:keyref`,
  `xs:unique`.
- **You need to validate while streaming.** The document is parsed in
  full first.

## FAQ

### Why does an unsupported construct get skipped rather than rejected?

Because a schema using one construct this crate lacks would otherwise
be unusable in full. Skipping means the surrounding rules still apply,
so a schema with an `xs:all` block validates everything else
correctly.

The cost is that a document can be reported valid when a construct
that was skipped would have rejected it. **Validation is incomplete,
not wrong** — and the distinction matters, so check
[Status](#status) before relying on a pass.

### Why is `xs:pattern` a hand-written engine?

Because XSD's regular expression dialect is not PCRE and not Rust's
`regex`. It has different anchoring semantics — the whole value must
match — its own character-class escapes, and Unicode block and category
escapes that neither crate spells the same way.

Using a general-purpose engine would mean translating one dialect into
another and being subtly wrong at the edges. The engine is a few
hundred lines and does exactly what the specification says.

### Is a schema reusable across documents?

Yes, and that is the intended shape. `parse_schema` is the expensive
half; `validate` is the half you repeat. A `Schema` is immutable after
parsing.

### Does it fetch schemas over the network?

No. `parse_schema` takes the schema's *text*. There is no code that
opens a file or a socket, which is also why `xs:import` and
`xs:include` are not supported — they name a location to fetch.

When they arrive, the shape will be a caller-supplied map from
location to content, never a fetch.

### What does a path like `/order/line[1]/qty` mean?

The `qty` child of the first `line` child of `order`. It is positional
so that it identifies one element and not a set — which is what you
need when the message is "this one is wrong".

### Does it validate the schema itself?

Partly. It rejects a schema that is not well-formed XML, reports what
it cannot understand, and enforces the structural rules XSD imposes on
schemas themselves — where `xs:annotation` may appear, which children
are mutually exclusive, that two element declarations of one name must
agree on their type, that a facet's value must belong to the type it
narrows, and that a restriction's content model must be a valid
restriction of its base.

It does not validate a schema against the full XSD
schema-for-schemas, which would be a second validator.

### How is this tested?

236 tests over schema parsing, every built-in type, every facet, the
pattern engine, the validator and the derivation relation. The XML
underneath carries the W3C XML conformance suite — 2,557 of 2,557
decided tests, zero panics.

**And the W3C XML Schema Test Suite**, `xsts-2007-06-20`, pinned by
SHA-256: **39,420 tests**, of which 95.0% of the decided ones pass,
with zero panics. This was the main gap in the crate's verification
until 0.0.6, and closing it is most of what 0.0.6 is.

A pass rate over that suite is only worth reporting because of how it
is counted. This crate implements a subset of XSD and skips what it
does not understand — and a schema whose constraints were all skipped
accepts every document, so agreeing with a test proves nothing. **A
test counts as a pass only when the schema is enforced in full.** On
the first run, 20,682 tests would otherwise have counted as passes
with nothing checked; that figure is published alongside the rate, in
`doc/CONFORMANCE.md`.

## Development

```bash
./scripts/gate.sh
```

That runs everything CI runs, in the order that fails fastest: format,
clippy, tests, rustdoc, the `#![forbid(unsafe_code)]` check, the
example, the W3C XSD conformance suite, the 95% coverage floor and an
MSRV build. It pins the toolchain rather than trusting
`rust-toolchain.toml`, because a `RUSTUP_TOOLCHAIN` in the environment
silently overrides that file and a lint that exists in one release and
not another then makes a green local run and a red CI one.

The conformance step is **skipped loudly** when the suite has not been
downloaded, and counts as a failure rather than vanishing. A skipped
conformance test is a passing one as far as `cargo test` is concerned,
and this crate's headline figure rests on that suite.

The individual steps, if you want them one at a time:

```bash
cargo test --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all --check
cargo bench --bench schema
cargo run --example validate
cargo run --release -p xmlschema-conformance --bin download
cargo test --release -p xmlschema-conformance
```

CI runs the same set on Linux, macOS and Windows.

## Security

XSD validation is normally applied to untrusted documents, which makes
the parser's threat model part of this crate's threat model. It
inherits `oxml`'s posture:

- **No entity expansion.** Only the five predefined entities and
  numeric character references are resolved, so XXE and billion-laughs
  are foreclosed by construction rather than by a flag.
- **No `unsafe`.** `#![forbid(unsafe_code)]`, enforced at compile time.

Report vulnerabilities privately — see [SECURITY.md](SECURITY.md).

## Documentation

- [API documentation](https://docs.rs/xmlschema)
- [CHANGELOG.md](CHANGELOG.md)

## Acknowledgements

- **[libxml2](https://gitlab.gnome.org/GNOME/libxml2)** — the
  reference implementation, and the yardstick for behaviour.
- **[W3C](https://www.w3.org/TR/xmlschema11-1/)** — for the XML Schema
  specification.
- **[python-xmlschema](https://github.com/sissaschool/xmlschema)** —
  proof that a readable, standalone XSD implementation is achievable.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
