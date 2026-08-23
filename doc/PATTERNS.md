<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# `xs:pattern`

## Why not use `regex`

XSD's regular expression dialect is not PCRE and not Rust's `regex`.
Three differences make translation the wrong approach:

**Anchoring.** An XSD pattern must match the **entire** value. `regex`
searches unless anchored, so every pattern would need wrapping — and
wrapping a pattern containing an alternation at the top level changes
what it means unless it is also parenthesised. `a|b` becomes `^(a|b)$`,
not `^a|b$`, and getting that wrong is silent.

**Escapes.** XSD has `\i` and `\c` for XML name-start and name
characters, which no general engine has. It lacks `\b`, `\B`, `\A`,
`\z`, backreferences and lookaround, so a pattern that happens to
contain them means something different in each.

**Character class subtraction.** `[a-z-[aeiou]]` is XSD syntax and
means "consonants". In `regex` it is a parse error or, worse, a
different class.

Translating one dialect into another means being subtly wrong at the
edges, and the edges of a validator are where it is load-bearing. The
engine here is a few hundred lines and implements what the
specification says.

## What it supports

- Literals, `.`, and the quantifiers `?`, `*`, `+`, `{n}`, `{n,}`,
  `{n,m}`
- Groups and alternation
- Character classes with ranges and negation
- Escapes: `\d`, `\D`, `\w`, `\W`, `\s`, `\S`, `\i`, `\I`, `\c`, `\C`,
  and escaped metacharacters
- Whole-value matching, which is the default and not a flag

## Cost

Backtracking, so a pathological pattern against a long value can be
expensive. Patterns come from a schema rather than from a document, so
in the usual threat model they are trusted input — but if you accept
schemas from elsewhere, that is a place to put a bound.

The document values matched against them are untrusted, and the
combination that matters for backtracking is a hostile *pattern*, not a
hostile value.
