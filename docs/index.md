---
title: xmlschema — W3C XML Schema (XSD) Validator
description: Pure Rust W3C XML Schema (XSD) validation library with zero unsafe code.
hide:
  - navigation
  - toc
---

<section class="dot-hero" markdown>

# xmlschema

<p class="tagline">A pure Rust W3C XML Schema (XSD 1.0) validation engine with zero unsafe code — fast, memory-safe, and deeply conformant.</p>

<div class="buttons">
  <a class="primary" href="https://docs.rs/xmlschema">API Docs →</a>
  <a href="https://github.com/sebastienrousseau/xmlschema">GitHub</a>
  <a href="PATTERNS/">Validation Patterns</a>
  <a href="COVERAGE/">XSD Coverage</a>
</div>

</section>

## What's inside

<div class="grid cards" markdown>

- :material-check-decagram:{ .lg .middle } **W3C XSD 1.0 conformance**

    ---

    Tested against 39,000+ official W3C XML Schema test suite cases for structural and facet validation.

    [→ Coverage Report](COVERAGE.md)

- :material-shield-check:{ .lg .middle } **Zero `unsafe` code**

    ---

    `#![forbid(unsafe_code)]` at crate root. Prevents parser crashes, buffer overflows, and untrusted schema exploits.

    [→ Assurance Case](ASSURANCE-CASE.md)

- :material-shape-plus:{ .lg .middle } **Type derivation & facets**

    ---

    Full simple and complex type validation: restrictions, extensions, enumerations, regex patterns, and min/max bounds.

    [→ Validation Patterns](PATTERNS.md)

- :material-puzzle-outline:{ .lg .middle } **Part of oxml suite**

    ---

    Integrates seamlessly with oxml's arena tree and streaming parser for maximum performance.

    [→ Roadmap](ROADMAP.md)

</div>

## Quick start

Add `xmlschema` to your project:

```toml
[dependencies]
xmlschema = "0.0.10"
```

Validate an XML document against an XSD schema:

```rust
use xmlschema::Schema;

fn main() -> Result<(), Box<dyn std.error.Error>> {
    let xsd = r#"
        <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
            <xs:element name="note">
                <xs:complexType>
                    <xs:sequence>
                        <xs:element name="to" type="xs:string"/>
                        <xs:element name="from" type="xs:string"/>
                    </xs:sequence>
                </xs:complexType>
            </xs:element>
        </xs:schema>
    "#;

    let schema = Schema::from_str(xsd)?;
    let valid_xml = "<note><to>Tove</to><from>Jani</from></note>";
    assert!(schema.validate(valid_xml).is_ok());

    Ok(())
}
```

## Where to next

- [**Validation Patterns**](PATTERNS.md) — Complex schemas, facets, and type constraints.
- [**XSD Coverage**](COVERAGE.md) — W3C test suite pass rates and component support.
- [**Assurance Case**](ASSURANCE-CASE.md) — Safety properties and resource exhaustion defenses.
- [**Testing**](TESTING.md) — Conformance harness execution and test suite tracking.
- [**Roadmap**](ROADMAP.md) — Schema compilation and XSD 1.1 feature plans.

## Current release

- Release notes: [GitHub Releases](https://github.com/sebastienrousseau/xmlschema/releases)
- Crates.io: [crates.io/crates/xmlschema](https://crates.io/crates/xmlschema)
- Repository: [sebastienrousseau/xmlschema](https://github.com/sebastienrousseau/xmlschema)
