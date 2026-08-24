<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# XSD coverage

## Implemented

- **Elements** with `type`, inline `complexType`, and cardinality via
  `minOccurs` / `maxOccurs` including `unbounded`
- **Content models**: `xs:sequence`, `xs:choice`, nested
- **Attributes**, including `use="required"` and `use="optional"`
- **Simple types**: nine built-ins, and restriction with nine facets
- **`xs:pattern`**, with its own engine — see [PATTERNS.md](PATTERNS.md)
- **Reports** listing every violation with a positional path

## Not implemented

- `xs:all`
- `xs:import`, `xs:include`, `xs:redefine`
- Identity constraints: `xs:key`, `xs:keyref`, `xs:unique`
- Complex-type derivation: `xs:extension`, `xs:restriction` on complex
  types
- Substitution groups
- `xs:any`, `xs:anyAttribute`
- XSD 1.1 in full: assertions, conditional type assignment

## What "skipped" means, and why it matters

An unsupported construct is **skipped**, not rejected. A schema
containing `xs:all` still validates everything outside it.

The alternative — refusing the whole schema — makes a schema with one
unsupported construct entirely unusable, which is worse for almost
every real user.

But the consequence has to be stated plainly:

> **A document can be reported valid when a skipped construct would
> have rejected it. Validation is incomplete, not wrong.**

So `is_valid()` means "no violation was found among the rules this
crate understands". If your schema uses anything in the "not
implemented" list, that is a weaker statement than it looks, and the
gap is exactly the constructs listed above.

A future release should carry the skipped constructs in the `Report`,
so `is_valid()` can be read alongside "and here is what was not
checked". That is the right fix and it is not done.

## No fetching, by design

`xs:import` and `xs:include` name a *location* to fetch. This crate
contains no code that opens a file or a socket — the same design that
forecloses XXE in the parser underneath.

When they are supported, the shape will be a caller-supplied map from
schema location to content. The caller decides what may be read; the
library never performs I/O.
