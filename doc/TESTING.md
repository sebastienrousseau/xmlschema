<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# Testing

```bash
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo run --example validate
```

207 tests over schema parsing, every built-in simple type, every
restriction facet, the pattern engine, the validator's content models
and cardinality, and the derivation relation.

## What the layer below provides

The XML parsing is `oxml`'s, and carries the heavier verification: the
W3C XML Conformance Test Suite at 2,520 of 2,557 decided tests with
**zero panics**, five fuzz targets, Miri, and property tests. See
<https://github.com/sebastienrousseau/oxml/blob/main/doc/TESTING.md>.

So a malformed schema or document cannot crash this crate, because it
cannot crash the parser.

## The conformance suite, and how it is counted

**The W3C XML Schema Test Suite**, release `xsts-2007-06-20`, pinned
by SHA-256: **39,420 tests**. Until 0.0.6 this section said there was
no such suite in use here and called that the main gap in the crate's
verification. It was, and closing it is most of what 0.0.6 is.

A pass rate over it is only worth reporting because of how it is
counted. This crate implements a subset of XSD and skips what it does
not understand -- so a schema whose constraints were all skipped
accepts every document, and a test expecting "valid" agrees with it
while nothing was enforced.

**So a test counts as a pass only when the schema is enforced in
full.** `support::unsupported` audits a schema against what this crate
actually enforces; any test with a gap is *unsupported* whatever the
answer, including when the answer is right. Those would-be passes are
counted separately and published, because their size is the whole
argument for measuring this way -- on the first run there were 20,682
of them, and a harness that counted them would have reported 74.2%
while measuring almost nothing.

```
overall  32980 pass, 1534 fail, 0 panic, 4841 unsupported, 65 blocked
         95.6% of 34514 decided (87.6% coverage of 39420)
```

Coverage is the figure to read first: it is the share of the suite
that produces an answer meaning anything, and a subset implementation
scores a high pass rate trivially by deciding little.

Run it with:

```bash
cargo run -p xmlschema-conformance --bin download
cargo run --release -p xmlschema-conformance --bin report
```

## What is still missing

Identity constraints (`key`, `keyref`, `unique`), `xs:import` and
`xs:include`, and substitution groups. Each is reported by
`support::unsupported` rather than silently skipped, so a schema using
one is counted as unenforceable rather than answered wrongly.

Treat [COVERAGE.md](COVERAGE.md) as the statement of what is checked,
and remember that an unsupported construct is skipped rather than
rejected: `is_valid()` means "no violation among the rules this crate
implements".
