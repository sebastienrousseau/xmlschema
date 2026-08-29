# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Every member of the [oxml](https://github.com/sebastienrousseau/oxml)
suite ships the same version number.

## [0.0.8] - 2026-08-29

### Added

- **The examples are measured against the public API.** The README
  claimed the examples covered it; they reached 15 of 31 public
  functions. Three examples close the gap -- datatype ordering and
  comparison, `xs:pattern` compilation, and particles, restriction
  and the reporting of constructs this crate does not enforce.

  The `Examples` job also named one example explicitly, so any added
  later was built and never run; it now discovers them.

### Security

- **`cargo audit` and `cargo deny` now actually run.** The Best
  Practices badge stated they ran against the RustSec advisory
  database. They did not.

  This repository was the worst case, because it *carried* a
  `deny.toml` -- which made the absence harder to see. That file was
  dead twice over: no workflow invoked it, and it was written for
  cargo-deny v1, whose `unlicensed`, `copyleft` and
  `allow-osi-fsf-free` keys were removed in v2. Its `allow` list held
  only `MPL-2.0`, so had it ever run against this crate's MIT/Apache
  dependency tree it would have denied everything.

- Every action pinned by commit SHA, branch coverage gated, CodeQL
  added, and the Developer Certificate of Origin enforced.

## [0.0.7] - 2026-08-28

### Changed

- Built on oxml 0.0.7, which reads a document from any `BufRead`. The
  suite ships one version number across all six crates.

### Fixed

- **The published-figures check read only `README.md`.** doc/TESTING.md
  drifted behind it exactly as the check was written to prevent,
  quoting 95.6% of 34,514 decided where the run gives 95.0% of 35,942,
  and 87.6% coverage where it gives 91.2%. The check now covers both
  documents, and was confirmed by reverting a figure and watching it
  fail.

- The test count stated in `README.md` and doc/TESTING.md said 207
  where the suite has 236. `tests/published_counts.rs` now checks it.

- oxml's conformance was quoted as 2,520 of 2,557 decided; it is
  2,557 of 2,557.

## [0.0.6] - 2026-08-26

### Added

- **A conformance measurement that means something.** The W3C XML
  Schema Test Suite, 39,420 tests, pinned by SHA-256. `support::unsupported`
  audits a schema against what this crate actually enforces, and a
  test counts as a pass only when the schema is enforced in *full* --
  a schema whose constraints were skipped accepts every document, so
  agreeing with the suite proves nothing. On the first run 20,682
  tests would otherwise have counted as passes with nothing checked.
- **Every XSD 1.0 built-in datatype**, each with its own rule.
  `xs:byte` accepted 999; the whole integer lattice was one unbounded
  type; `xs:NCName`, `xs:ID` and `xs:language` were all `xs:string`;
  and `xs:duration`, `xs:time` and the gregorian types did not resolve
  at all.
- **`xs:list` and `xs:union`.** Length facets count *items* on a list.
- **`xs:any`, `xs:anyAttribute`, `xs:all`, `xs:group`,
  `xs:attributeGroup`, `xs:complexContent`, `@ref`, `@fixed`,
  `@nillable` and `use="prohibited"`.**
- **`whiteSpace`, `totalDigits` and `fractionDigits`.**
- **XSD's own structural rules**: annotation placement, mutually
  exclusive children, Element Declarations Consistent, facet values
  belonging to their base type, and duplicate attribute names.
- **Particle Valid (Restriction)** -- the subsumption relation between
  content models, in `derive`.
- **Benchmarks**, in three groups. There were none, so no performance
  claim about this crate could be checked.

### Fixed

- **Patterns were compiled once per value rather than once per
  schema**, which cost more than matching did: `validate/faceted_1000`
  886 us to 290 us.
- **XSD's regex dialect**: `\i`, `\c`, `\p{...}`, and character class
  subtraction `[A-[B]]` -- which the specification uses to define its
  own name types, so no `NCName` pattern compiled.
- **`\d` is `\p{Nd}`**, not `[0-9]`, and `\s` is four characters
  rather than Unicode whitespace.
- **Bounds apply to dates, times and durations.** Stored as `f64`,
  every temporal bound was silently dropped.
- **Decimals compare exactly**, not through an `f64` that cannot tell
  `999999999999999998` from `999999999999999999`.
- **A schema declaring no top-level element is valid** -- it exists to
  be imported -- and an unresolvable `ref` is unenforceable rather
  than invalid.
- **A schema that inlines a type twice per level is refused** rather
  than expanded: 24 levels is sixteen million particles from a few
  kilobytes.

### Changed

- `Facets::pattern` holds a compiled `Pattern`; bounds are held
  lexically; `Content::Simple` is boxed.

  Measured against the suite: pass rate 71.7% to **95.6%**, coverage of
  the suite 27.0% to **87.6%**, zero panics throughout.

## [0.0.5] - 2026-08-24

### Changed

- Built on oxml 0.0.5, which completes `XPath` 1.0: all thirteen axes
  and all 27 functions.

  **One behaviour change reaches expressions passed through this
  crate.** A function name outside the specification's library, or a
  call with the wrong number of arguments, used to compile and evaluate
  to an empty node-set. Both are now compile errors, reported with an
  offset. `starts-with("abc")` answered `true` before, because the
  absent argument read as the empty string.

  Six functions that previously answered `""` now work:
  `substring-before`, `substring-after`, `translate`, `name`, `id` and
  `lang`. So do the `following`, `preceding` and `namespace` axes.

## [0.0.4] - 2026-08-24

### Changed

- Built against `oxml` 0.0.4, which resolves XPath namespace prefixes,
  normalises line endings and attribute values, and reaches 98.6% of
  decided W3C conformance tests. This crate has no expression surface,
  so nothing here changes for callers.

## [0.0.3] - 2026-08-22

### Added

- **XSD validation.** The crate now does what its name says. Parse an
  `.xsd` into a schema model, validate a document against it, and get
  back every violation with the path to the element it concerns.

- **A schema parser** covering elements, `xs:sequence`, `xs:choice`,
  cardinality (`minOccurs` / `maxOccurs`, including `unbounded`),
  attributes with `use="required"`, named top-level simple types, and
  inline simple and complex types.

- **Nine built-in simple types** — string, boolean, decimal, integer,
  non-negative integer, double, date, dateTime, anyURI — and nine
  restriction facets: `enumeration`, `pattern`, `length`, `minLength`,
  `maxLength`, `minInclusive`, `maxInclusive`, `minExclusive`,
  `maxExclusive`.

- **A regular-expression engine for `xs:pattern`.** XSD patterns are a
  dialect of their own: always anchored, no capture groups, and their
  own character-class escapes. Rather than depend on a general regex
  crate and then explain which parts do not apply, this implements the
  subset XSD defines — literals, `.`, classes with ranges and
  negation, `\d \D \w \W \s \S`, groups, alternation, and the `? * +`
  and `{n}` `{n,}` `{n,m}` quantifiers.

- **Diagnostics that collect rather than short-circuit.** A caller
  fixing a document wants the whole list, not to re-run the validator
  after each edit. Each violation carries a path such as
  `/invoice/line[2]/@currency` and a message naming what was expected.

### Changed

- **Depends on [`oxml`](https://github.com/sebastienrousseau/oxml)**
  for parsing and tree traversal. A validator needs a document model
  before it needs validation rules, which is where that work went
  first.

- `#![forbid(unsafe_code)]`, inheriting oxml's posture: no entity
  expansion, so XXE and billion-laughs are foreclosed by construction
  in a crate whose whole job is reading untrusted documents.

### Removed

- The 935-line private data model from `0.0.1`, which had **no public
  API** — every type in it was private, so nothing could be
  constructed or called from outside the crate.

- The `build.rs` and bundled `XMLSchema.xsd` it used, both unreferenced
  by the rewrite.

## [0.0.1] - 2023-02-18

### Added

- Initial release: a partial data model of the XSD schema language,
  with no public API.

[0.0.3]: https://github.com/sebastienrousseau/xmlschema/releases/tag/v0.0.3
