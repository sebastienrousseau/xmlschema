<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# Testing

```bash
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo run --example validate
```

97 tests over schema parsing, each built-in simple type, each
restriction facet, the pattern engine, and the validator's content
models and cardinality.

## What the layer below provides

The XML parsing is `oxml`'s, and carries the heavier verification: the
W3C XML Conformance Test Suite at 2,394 of 2,557 decided tests with
**zero panics**, five fuzz targets, Miri, and property tests. See
<https://github.com/sebastienrousseau/oxml/blob/main/doc/TESTING.md>.

So a malformed schema or document cannot crash this crate, because it
cannot crash the parser.

## The gap, stated plainly

**There is no XSD conformance suite in use here.**

`oxml` is measured against 2,585 published tests with a ratcheted
baseline, and the number is reported with its denominator. This crate
has 97 tests that its authors thought of.

That is a materially weaker form of evidence, and it is the main thing
missing from this crate's verification. The W3C XML Schema Test
Collection exists and running it is the right next step — it would
convert "we tested what we thought of" into a number with a
denominator, and it would very likely find things.

Until then, treat [COVERAGE.md](COVERAGE.md) as the honest statement of
what is checked, and remember that an unsupported construct is skipped
rather than rejected: `is_valid()` means "no violation among the rules
this crate implements".
