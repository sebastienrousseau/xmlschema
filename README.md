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
- [Install](#install) — once there is something to install

**Reference**

- [Why this crate exists](#why-this-crate-exists) — the gap it fills
- [The oxml ecosystem](#the-oxml-ecosystem) — where this fits
- [Ecosystem comparison](#ecosystem-comparison) — XSD support in Rust
- [Planned capabilities](#planned-capabilities) — the roadmap

**Practical**

- [Development](#development)
- [Security](#security)
- [Documentation](#documentation)
- [Acknowledgements](#acknowledgements)
- [License](#license)

---

## Status

| | State |
|---|---|
| Schema parsing | ✅ elements, sequence, choice, cardinality, attributes |
| Simple types | ✅ nine built-ins, nine restriction facets |
| `xs:pattern` | ✅ own engine, XSD dialect |
| Diagnostics | ✅ every violation, each with a path |
| Tests | ✅ 27 |
| `xs:all` | ✗ |
| `xs:import` / `include` | ✗ |
| Identity constraints | ✗ |
| Complex-type derivation | ✗ |

An unsupported construct is skipped rather than rejected: the
surrounding rules still apply, so a schema using `xs:all` validates
everything else correctly instead of failing wholesale.

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
| [`oxml-cli`](https://github.com/sebastienrousseau/oxml-cli) | Command-line querying and formatting | Planned |
| [`oxml-lsp`](https://github.com/sebastienrousseau/oxml-lsp) | Language server | Planned |
| [`oxml-mcp`](https://github.com/sebastienrousseau/oxml-mcp) | Model Context Protocol server | Planned |
| [`oxml-wasm`](https://github.com/sebastienrousseau/oxml-wasm) | WebAssembly bindings | Planned |
| **`xmlschema`** | **XSD validation** | **Being rewritten** |

This crate keeps its published name rather than being folded into
`oxml`. The name means XSD validation specifically, and repurposing it
into a general toolkit would have handed existing users something
entirely different under a name they already depend on.

## Ecosystem comparison

| Crate | XSD validation | Pure Rust | WASM | Last release |
|---|---|---|---|---|
| **`xmlschema`** | planned | ✅ | ✅ | 2023 (unusable) |
| `libxml` | ✅ | ✗ (C-FFI) | ✗ | active |
| `quick-xml` | ✗ | ✅ | ✅ | active |
| `roxmltree` | ✗ | ✅ | ✅ | active |
| `xot` | ✗ | ✅ | ✅ | 2025 |

## Planned capabilities

In order:

1. **Schema parsing** — read an `.xsd` into a usable model, built on
   `oxml`'s tree.
2. **Structural validation** — elements, attributes, cardinality,
   sequence/choice/all.
3. **Simple type validation** — the built-in datatypes, restrictions,
   patterns, enumerations.
4. **Complex types** — extension, restriction, mixed content.
5. **Diagnostics** — every violation reported with an element path and
   a reason, so a caller can fix all of them in one pass rather than
   probing one failure at a time.

Import mechanisms (`xs:import`, `xs:include`, `xs:redefine`) come after
the core is correct, because they multiply the surface without adding
validation power.

## Development

```bash
git clone https://github.com/sebastienrousseau/xmlschema
cd xmlschema
cargo test
```

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
