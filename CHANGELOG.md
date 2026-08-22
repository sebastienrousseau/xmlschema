# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- **Repositioned as the schema member of the
  [oxml](https://github.com/sebastienrousseau/oxml) suite.** The name
  means XSD validation, and that is what this crate will be — rather
  than being repurposed into a general XML toolkit, which would have
  handed existing users something entirely different under a name they
  already depend on.

- **README rewritten** to the suite's standard, and made honest about
  the state of the crate.

### Notes

`0.0.1` is published and **exposes no public API**. Every type in it —
`XmlSchema`, `Element`, `ComplexType`, `SimpleType`, `Attribute` — is
private, so nothing can be constructed or called from outside the
crate. It compiles, and does nothing.

That is recorded here rather than quietly fixed in a later release,
because 2,658 downloads have already resolved to it.

## [0.0.1] - 2023-02-18

### Added

- Initial release: a partial data model of the XSD schema language.
