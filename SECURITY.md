# Security Policy

## Supported versions

xmlschema is pre-1.0. Only the most recent `0.0.x` release receives security
fixes.

## Reporting a vulnerability

Please report privately, not as a public issue:

- GitHub's [private vulnerability reporting](https://github.com/sebastienrousseau/xmlschema/security/advisories/new)
- or email <sebastian.rousseau@gmail.com>

You should get an acknowledgement within 72 hours.

## Threat model

XSD validators inherit their parser's threat model. XML parsers have a well-known attack surface, and xmlschema's design
forecloses most of it rather than mitigating it:

**Entity expansion (XXE, billion laughs).** xmlschema resolves only the five
predefined entities and numeric character references. External and
custom entities are *not* resolved — a document declaring
`<!ENTITY xxe SYSTEM "file:///etc/passwd">` is rejected rather than
expanded. There is no option to enable it, so there is no way to
configure the vulnerability back in.

**Unbounded recursion.** Deeply nested documents are parsed with
recursion. Extremely deep input can exhaust the stack; if you parse
untrusted documents of unbounded depth, run the parse on a thread with
a known stack size.

**Memory safety.** `#![forbid(unsafe_code)]` is enforced at compile
time, so the class of memory-corruption bugs that has historically
affected C XML parsers cannot occur here.

## What is not a vulnerability

- Rejecting a document other parsers accept, when the document is not
  well-formed. Please still report it — it is a bug, just not a
  security one.
