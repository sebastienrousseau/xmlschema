# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Every member of the [oxml](https://github.com/sebastienrousseau/oxml)
suite ships the same version number.

## [Unreleased]

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
