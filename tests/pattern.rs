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

// ---------------------------------------------------------------------
// Malformed patterns, and the constructs that had no coverage.
//
// A schema author's `xs:pattern` is data, not code: a bad one must be
// reported, never panic and never silently match everything.
// ---------------------------------------------------------------------

#[test]
fn a_malformed_pattern_is_an_error_with_a_message() {
    for bad in [
        "(unclosed",
        "[unterminated",
        "a{",
        "a{2",
        "a{x}",
        "a{2,x}",
        "*",
        "+",
        "?",
        "a**",
        ")",
        "[a-",
    ] {
        let e = Pattern::compile(bad);
        assert!(e.is_err(), "`{bad}` compiled but should not have");
        let msg = e.expect_err("checked above").to_string();
        assert!(!msg.is_empty(), "`{bad}` produced an empty message");
    }
}

#[test]
fn an_empty_alternative_is_legal() {
    // `a|` means "a, or nothing" — Python and libxml2 both accept it,
    // and it is not the same class of mistake as a stray quantifier.
    let p = Pattern::compile("a|").expect("valid");
    assert!(p.matches("a"));
    assert!(p.matches(""));
    assert!(!p.matches("aa"));
}

#[test]
fn a_quantifier_must_be_escaped_to_mean_itself() {
    // Parsing a bare `*` as a literal would turn a typo into a pattern
    // that silently matches the wrong values instead of failing.
    assert!(Pattern::compile("*").is_err());
    let escaped = Pattern::compile(r"a\*b").expect("escaped is valid");
    assert!(escaped.matches("a*b"));
    assert!(!escaped.matches("ab"));
}

#[test]
fn an_error_message_names_the_offending_position() {
    let e = Pattern::compile(")").expect_err("unbalanced");
    let msg = e.to_string();
    assert!(
        msg.contains("position") || msg.contains("unexpected"),
        "{msg}"
    );
}

#[test]
fn an_empty_pattern_matches_only_the_empty_string() {
    let p = Pattern::compile("").expect("empty is a valid pattern");
    assert!(p.matches(""));
    assert!(!p.matches("a"));
}

#[test]
fn dot_matches_any_single_character() {
    let p = Pattern::compile("a.c").expect("valid");
    assert!(p.matches("abc"));
    assert!(p.matches("a c"));
    assert!(p.matches("a9c"));
    assert!(!p.matches("ac"), "dot must consume exactly one character");
    assert!(!p.matches("abbc"));
}

#[test]
fn bounded_repetition_honours_both_ends() {
    let p = Pattern::compile("a{2,4}").expect("valid");
    assert!(!p.matches("a"));
    assert!(p.matches("aa"));
    assert!(p.matches("aaaa"));
    assert!(!p.matches("aaaaa"));

    let exact = Pattern::compile("a{3}").expect("valid");
    assert!(!exact.matches("aa"));
    assert!(exact.matches("aaa"));
    assert!(!exact.matches("aaaa"));

    let open = Pattern::compile("a{2,}").expect("valid");
    assert!(!open.matches("a"));
    assert!(open.matches("aa"));
    assert!(open.matches("aaaaaaaa"));
}

#[test]
fn a_pattern_is_anchored_at_both_ends() {
    // XSD patterns match the whole value; a substring match would let
    // through everything the author meant to exclude.
    let p = Pattern::compile("abc").expect("valid");
    assert!(p.matches("abc"));
    assert!(!p.matches("xabc"));
    assert!(!p.matches("abcx"));
    assert!(!p.matches("xabcx"));
}

#[test]
fn escapes_inside_a_class_are_recognised() {
    // `[\d]` and `[\s]` are the common forms in real schemas, and the
    // in-class escape table is separate from the top-level one.
    assert!(m(r"[\d]+", "123"));
    assert!(!m(r"[\d]+", "abc"));
    assert!(m(r"[\s]", " "));
    assert!(m(r"[\w]+", "ab_1"));
}

#[test]
fn negated_class_escapes_are_recognised() {
    assert!(m(r"\D+", "abc"));
    assert!(!m(r"\D+", "123"));
    assert!(m(r"\W+", "!!!"));
    assert!(!m(r"\W+", "abc"));
    assert!(m(r"\S+", "abc"));
    assert!(!m(r"\S+", "   "));
    assert!(m(r"[\D]+", "abc"));
    assert!(m(r"[\W]+", "!!"));
    assert!(m(r"[\S]+", "ab"));
}

#[test]
fn whitespace_escapes_match_their_characters() {
    assert!(m(r"a\nb", "a\nb"));
    assert!(m(r"a\tb", "a\tb"));
    assert!(m(r"a\rb", "a\rb"));
    assert!(m(r"[\n\t\r]", "\n"));
    assert!(m(r"[\n\t\r]", "\t"));
    assert!(m(r"[\n\t\r]", "\r"));
}

#[test]
fn an_escaped_metacharacter_is_a_literal_in_a_class() {
    assert!(m(r"[\.\-]+", ".-"));
    assert!(!m(r"[\.\-]+", "a"));
}

#[test]
fn a_trailing_backslash_is_an_error_everywhere() {
    assert!(Pattern::compile("a\\").is_err());
    assert!(Pattern::compile("[a\\").is_err());
}

#[test]
fn an_unterminated_range_is_an_error() {
    assert!(Pattern::compile("[a-").is_err());
    assert!(Pattern::compile("[a-z").is_err());
}

#[test]
fn an_invalid_upper_bound_is_an_error() {
    assert!(Pattern::compile("a{2,x}").is_err());
    assert!(Pattern::compile("a{2,").is_err());
}

#[test]
fn a_group_must_be_closed() {
    assert!(Pattern::compile("(ab").is_err());
    assert!(Pattern::compile("(a|b").is_err());
    assert!(Pattern::compile("(ab)").is_ok());
}

#[test]
fn an_empty_group_and_an_empty_class_behave_predictably() {
    assert!(Pattern::compile("()").expect("valid").matches(""));
    // An unterminated class is an error rather than an empty one.
    assert!(Pattern::compile("[").is_err());
}

#[test]
fn a_pattern_ending_mid_construct_is_an_error_not_a_panic() {
    for bad in ["(", "[", "a{", "a{1", "\\", "[^", "[a-z"] {
        assert!(Pattern::compile(bad).is_err(), "`{bad}` should not compile");
    }
}

/// The XSD dialect has escapes no other regex flavour has.
#[test]
fn the_xsd_specific_escapes_are_supported() {
    // `\i` is a character a name may start with, `\c` one it may
    // contain. Neither exists in PCRE.
    let start = Pattern::compile(r"\i\c*").expect("compiles");
    assert!(start.matches("abc"));
    assert!(start.matches("_a-b.c"));
    assert!(!start.matches("1abc"), "a name may not start with a digit");
    assert!(!start.matches("a b"), "a space is not a name character");

    // Their negations.
    let not_start = Pattern::compile(r"\I").expect("compiles");
    assert!(not_start.matches("1"));
    assert!(!not_start.matches("a"));

    let not_char = Pattern::compile(r"\C").expect("compiles");
    assert!(not_char.matches(" "));
    assert!(!not_char.matches("a"));
}

/// A Unicode category this engine cannot decide exactly is a compile
/// error rather than a guess.
#[test]
fn unicode_categories_are_a_whitelist_not_a_guess() {
    for ok in [r"\p{L}+", r"\p{Lu}", r"\p{Ll}", r"\p{N}", r"\p{Nd}+"] {
        assert!(Pattern::compile(ok).is_ok(), "{ok}");
    }
    // Approximating these would match the wrong set, and a pattern
    // that quietly matches the wrong set is worse than one that
    // refuses to compile: a refusal is reported as unenforceable.
    for refused in [r"\p{Lt}", r"\p{IsBasicLatin}", r"\p{Sc}", r"\p{Mn}"] {
        let e = Pattern::compile(refused).expect_err("{refused}");
        assert!(e.to_string().contains("not supported"), "{refused}: {e}");
    }
    // Malformed forms are errors too.
    assert!(Pattern::compile(r"\pL").is_err(), "no brace");
    assert!(Pattern::compile(r"\p{L").is_err(), "unterminated");

    let letters = Pattern::compile(r"\p{L}+").expect("compiles");
    assert!(letters.matches("abcXYZ"));
    assert!(!letters.matches("abc1"));
}

#[test]
fn a_pattern_must_match_the_whole_value() {
    // XSD anchors implicitly at both ends, unlike most engines.
    let p = Pattern::compile("abc").expect("compiles");
    assert!(p.matches("abc"));
    assert!(!p.matches("xabc"), "a leading extra");
    assert!(!p.matches("abcx"), "a trailing extra");
}

#[test]
fn alternation_and_grouping_combine() {
    let p = Pattern::compile("(ab|cd)+e").expect("compiles");
    assert!(p.matches("abe"));
    assert!(p.matches("cde"));
    assert!(p.matches("abcdabe"));
    assert!(!p.matches("ace"));
}

#[test]
fn malformed_patterns_are_rejected_rather_than_guessed() {
    for bad in ["(", "[", "a{", "a{2,1x}", "*", "+abc", r"\"] {
        assert!(Pattern::compile(bad).is_err(), "`{bad}` should not compile");
    }
}

/// Each supported category decides the characters it names.
#[test]
fn the_supported_categories_match_their_characters() {
    let cases: &[(&str, &str, &str)] = &[
        // (pattern, matches, does not match)
        (r"\p{L}+", "abcDEF", "abc1"),
        (r"\p{Lu}+", "ABC", "abc"),
        (r"\p{Ll}+", "abc", "ABC"),
        (r"\p{N}+", "123", "abc"),
        (r"\p{Nd}+", "456", "xyz"),
        (r"\p{Zs}+", " ", "a"),
        (r"\p{Z}+", " ", "a"),
        (r"\p{C}+", "\u{7}", "a"),
    ];
    for (pattern, yes, no) in cases {
        let p = Pattern::compile(pattern)
            .unwrap_or_else(|e| panic!("{pattern}: {e}"));
        assert!(p.matches(yes), "{pattern} should match {yes:?}");
        assert!(!p.matches(no), "{pattern} should not match {no:?}");
    }
    // And a negated category.
    let not_digit = Pattern::compile(r"\P{Nd}+").expect("compiles");
    assert!(not_digit.matches("abc"));
    assert!(!not_digit.matches("123"));
}

#[test]
fn a_malformed_quantifier_bound_is_an_error() {
    for bad in ["a{2,x}", "a{x}", "a{2,3", "a{,2}"] {
        assert!(Pattern::compile(bad).is_err(), "`{bad}`");
    }
}

/// XSD writes `[A-[B]]` for "in A but not in B".
///
/// No other regex dialect has it, and the specification uses it to
/// define its own name types: `[\i-[:]]` is a name start that is not a
/// colon, which is exactly an `NCName` start.
#[test]
fn a_character_class_may_subtract_another() {
    let ncname = Pattern::compile(r"[\i-[:]][\c-[:]]*").expect("compiles");
    assert!(ncname.matches("abc"));
    assert!(ncname.matches("_a-b.c"));
    assert!(!ncname.matches("a:b"), "a colon is subtracted out");
    assert!(!ncname.matches(":a"), "including at the start");
    assert!(!ncname.matches("1a"), "still a name start");

    // Subtracting from a plain range.
    let consonant = Pattern::compile("[a-z-[aeiou]]+").expect("compiles");
    assert!(consonant.matches("bcd"));
    assert!(!consonant.matches("abc"), "a is subtracted");

    // Subtraction binds before negation.
    let negated = Pattern::compile("[^a-z-[aeiou]]+").expect("compiles");
    assert!(negated.matches("aei"), "vowels are outside the group");
    assert!(!negated.matches("bcd"));

    // A quantifier applies to the whole subtracted class.
    let three = Pattern::compile("[a-z-[aeiou]]{3}").expect("compiles");
    assert!(three.matches("bcd"));
    assert!(!three.matches("bc"));

    // And a malformed one is an error rather than a guess.
    assert!(Pattern::compile("[a-[b]").is_err(), "unterminated");
}
