// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 xmlschema. All rights reserved.

//! The `xs:pattern` engine.

use xmlschema::Pattern;

fn m(pattern: &str, value: &str) -> bool {
    Pattern::compile(pattern)
        .unwrap_or_else(|e| panic!("`{pattern}` failed to compile: {e}"))
        .matches(value)
}

#[test]
fn literals_match_exactly() {
    assert!(m("abc", "abc"));
    assert!(!m("abc", "abd"));
}

/// XSD patterns are anchored at both ends, unlike Perl-style regexes.
/// A partial match is not a match.
#[test]
fn patterns_are_anchored() {
    assert!(!m("abc", "xabcx"));
    assert!(!m("abc", "abcd"));
    assert!(!m("b", "abc"));
}

#[test]
fn character_classes_and_ranges() {
    assert!(m("[abc]", "b"));
    assert!(!m("[abc]", "d"));
    assert!(m("[a-z]+", "hello"));
    assert!(!m("[a-z]+", "Hello"));
    assert!(m("[^0-9]+", "abc"));
    assert!(!m("[^0-9]+", "a1c"));
}

#[test]
fn escapes_behave_as_classes() {
    assert!(m(r"\d{3}", "123"));
    assert!(!m(r"\d{3}", "12a"));
    assert!(m(r"\w+", "a_1"));
    assert!(m(r"\s", " "));
    assert!(m(r"\D+", "abc"));
}

#[test]
fn quantifiers_bound_repetition() {
    assert!(m("a?", ""));
    assert!(m("a?", "a"));
    assert!(!m("a?", "aa"));
    assert!(m("a*", ""));
    assert!(m("a*", "aaaa"));
    assert!(!m("a+", ""));
    assert!(m("a+", "a"));
    assert!(m("a{2}", "aa"));
    assert!(!m("a{2}", "a"));
    assert!(m("a{2,}", "aaa"));
    assert!(m("a{2,3}", "aa"));
    assert!(!m("a{2,3}", "aaaa"));
}

#[test]
fn alternation_tries_each_branch() {
    assert!(m("cat|dog", "cat"));
    assert!(m("cat|dog", "dog"));
    assert!(!m("cat|dog", "cow"));
}

#[test]
fn groups_compose_with_quantifiers() {
    assert!(m("(ab)+", "abab"));
    assert!(!m("(ab)+", "aba"));
    assert!(m("(a|b){3}", "aba"));
}

/// The realistic case: an ISBN, a postcode, a currency code.
#[test]
fn real_world_patterns() {
    assert!(m(r"\d{3}-\d{10}", "978-0441013593"));
    assert!(!m(r"\d{3}-\d{10}", "978-044101359"));
    assert!(m("[A-Z]{3}", "GBP"));
    assert!(!m("[A-Z]{3}", "GB"));
    // UK postcodes: the outward code may carry a trailing letter,
    // which `[A-Z]{1,2}\d{1,2}` alone does not allow. Cross-checked
    // against Python's `re.fullmatch`, which agrees on both.
    assert!(m(r"[A-Z]{1,2}\d{1,2} ?\d[A-Z]{2}", "SW1 1AA"));
    assert!(!m(r"[A-Z]{1,2}\d{1,2} ?\d[A-Z]{2}", "SW1A 1AA"));
    assert!(m(r"[A-Z]{1,2}\d{1,2}[A-Z]? ?\d[A-Z]{2}", "SW1A 1AA"));
}

/// Backtracking must terminate. A quantified group that can match
/// nothing would otherwise loop forever.
#[test]
fn zero_width_repetition_terminates() {
    assert!(m("(a*)*", "aaa"));
    assert!(m("(a?)*", ""));
}

#[test]
fn malformed_patterns_report_rather_than_panic() {
    assert!(Pattern::compile("[abc").is_err());
    assert!(Pattern::compile("(ab").is_err());
    assert!(Pattern::compile("a{2").is_err());
    assert!(Pattern::compile(r"a\").is_err());
}

#[test]
fn the_source_is_preserved_for_diagnostics() {
    let p = Pattern::compile(r"\d+").expect("compiles");
    assert_eq!(p.source(), r"\d+");
}
