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

> [!WARNING]
> **This crate is being rewritten and is not usable today.**
>
> `0.0.1` was published in February 2023 and exposes **no public
> API** — every type in it is private, so nothing can be constructed,
> called, or validated from outside the crate. If you depend on it
> today you get a crate that compiles and does nothing.
>
> It is being rebuilt as the schema member of the
> [oxml](https://github.com/sebastienrousseau/oxml) suite. Until then,
> use [`libxml`](https://crates.io/crates/libxml) if you need XSD
> validation in Rust.

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

Honestly: **not usable**.

| | State |
|---|---|
| Published version | `0.0.1`, February 2023 |
| Public API | **None** — every type is private |
| Validation | Not implemented |
| Parser | Partial XSD data model, no wiring |
| Tests | None |
| CI | None |

The crate holds a partial data model of the XSD schema language —
`Element`, `ComplexType`, `SimpleType`, `Attribute` and friends — but
they are not `pub`, there is no parser that populates them, and there
is no validator that consumes them.

That is being fixed rather than papered over. The rewrite depends on
[`oxml`](https://github.com/sebastienrousseau/oxml) for parsing and
tree traversal, which is where the work went first: a validator needs
a document model before it needs validation rules.

## Install

Not yet. When the rewrite lands:

```toml
[dependencies]
xmlschema = "0.0.X"
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
