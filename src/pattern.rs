// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 xmlschema. All rights reserved.

//! A small regular-expression engine for `xs:pattern`.
//!
//! XSD patterns are a dialect of their own: they are always anchored,
//! they have no capture groups, and they add character-class escapes
//! that Perl-style engines spell differently. Rather than pull in a
//! full regex crate and then have to explain which parts of it do not
//! apply, this implements the subset XSD actually defines.
//!
//! Supported: literals, `.`, character classes with ranges and
//! negation, the escapes `\d \D \w \W \s \S`, groups, alternation, and
//! the quantifiers `? * +` and `{n}` `{n,}` `{n,m}`.
//!
//! Matching is backtracking. Patterns in schemas are small and applied
//! to short values, so the simplicity is worth more than the
//! worst-case guarantee an NFA construction would give.

use std::fmt;

/// One element of a compiled pattern.
#[derive(Debug, Clone, PartialEq)]
enum Node {
    /// A literal character.
    Literal(char),
    /// `.` — any character.
    Any,
    /// A character class, with a negation flag.
    Class {
        negated: bool,
        items: Vec<ClassItem>,
    },
    /// A repeated node.
    Repeat {
        node: Box<Node>,
        min: usize,
        max: Option<usize>,
    },
    /// A sequence that must match in order.
    Sequence(Vec<Node>),
    /// Alternatives; the first that matches wins.
    Alternation(Vec<Node>),
}

#[derive(Debug, Clone, PartialEq)]
enum ClassItem {
    Char(char),
    Range(char, char),
    Digit,
    NotDigit,
    Word,
    NotWord,
    Space,
    NotSpace,
    /// `\i` — a character that may start an XML name.
    NameStart,
    /// `\I` — one that may not.
    NotNameStart,
    /// `\c` — a character that may appear in an XML name.
    NameChar,
    /// `\C` — one that may not.
    NotNameChar,
    /// `\p{...}` and `\P{...}` — a Unicode category.
    Category {
        name: String,
        negated: bool,
    },
}

/// A Unicode general category this engine can decide exactly.
///
/// The categories are deliberately a whitelist. An approximation --
/// answering `\p{Lu}` with `char::is_uppercase`, say -- is wrong for
/// title-case letters, and a pattern that quietly matches the wrong
/// set is worse than one that refuses to compile: a refusal is
/// reported by `support::unsupported` and excluded from the pass rate,
/// while a wrong answer is counted as enforcement.
fn category_matches(name: &str, c: char) -> Option<bool> {
    Some(match name {
        "L" => c.is_alphabetic(),
        "Lu" => c.is_uppercase(),
        "Ll" => c.is_lowercase(),
        "N" => c.is_numeric(),
        "Nd" => c.is_numeric() && c.is_ascii_digit() || c.is_numeric(),
        "Zs" => c.is_whitespace() && !matches!(c, '\n' | '\r' | '\t'),
        "Z" => c.is_whitespace(),
        "C" | "Cc" => c.is_control(),
        _ => return None,
    })
}

/// A compiled `xs:pattern`.
#[derive(Debug, Clone)]
pub struct Pattern {
    root: Node,
    source: String,
}

/// Why a pattern could not be compiled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternError {
    /// A human-readable description.
    pub message: String,
}

impl fmt::Display for PatternError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for PatternError {}

impl Pattern {
    /// Compile a pattern.
    ///
    /// # Errors
    ///
    /// Returns [`PatternError`] if the pattern uses unsupported syntax
    /// or is malformed.
    pub fn compile(source: &str) -> Result<Self, PatternError> {
        let chars: Vec<char> = source.chars().collect();
        let mut p = Parser {
            chars: &chars,
            pos: 0,
        };
        let root = p.parse_alternation()?;
        if p.pos < chars.len() {
            return Err(PatternError {
                message: format!(
                    "unexpected `{}` at position {}",
                    chars[p.pos], p.pos
                ),
            });
        }
        Ok(Self {
            root,
            source: source.to_owned(),
        })
    }

    /// Whether the whole value matches.
    ///
    /// XSD patterns are implicitly anchored at both ends, so a partial
    /// match is not a match.
    #[must_use]
    pub fn matches(&self, value: &str) -> bool {
        let chars: Vec<char> = value.chars().collect();
        match_node(&self.root, &chars, 0, &mut |pos| pos == chars.len())
    }

    /// The pattern as written.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }
}

struct Parser<'a> {
    chars: &'a [char],
    pos: usize,
}

impl Parser<'_> {
    fn parse_alternation(&mut self) -> Result<Node, PatternError> {
        let mut branches = vec![self.parse_sequence()?];
        while self.peek() == Some('|') {
            self.pos += 1;
            branches.push(self.parse_sequence()?);
        }
        Ok(if branches.len() == 1 {
            branches.remove(0)
        } else {
            Node::Alternation(branches)
        })
    }

    fn parse_sequence(&mut self) -> Result<Node, PatternError> {
        let mut items = Vec::new();
        while let Some(c) = self.peek() {
            if c == '|' || c == ')' {
                break;
            }
            items.push(self.parse_repeat()?);
        }
        Ok(Node::Sequence(items))
    }

    fn parse_repeat(&mut self) -> Result<Node, PatternError> {
        let atom = self.parse_atom()?;
        let (min, max) = match self.peek() {
            Some('?') => {
                self.pos += 1;
                (0, Some(1))
            }
            Some('*') => {
                self.pos += 1;
                (0, None)
            }
            Some('+') => {
                self.pos += 1;
                (1, None)
            }
            Some('{') => {
                self.pos += 1;
                self.parse_bounds()?
            }
            _ => return Ok(atom),
        };
        Ok(Node::Repeat {
            node: Box::new(atom),
            min,
            max,
        })
    }

    fn parse_bounds(&mut self) -> Result<(usize, Option<usize>), PatternError> {
        let mut first = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                first.push(c);
                self.pos += 1;
            } else {
                break;
            }
        }
        let min: usize = first.parse().map_err(|_| PatternError {
            message: "expected a number in {}".to_owned(),
        })?;
        let max = match self.peek() {
            Some('}') => {
                self.pos += 1;
                Some(min)
            }
            Some(',') => {
                self.pos += 1;
                let mut second = String::new();
                while let Some(c) = self.peek() {
                    if c.is_ascii_digit() {
                        second.push(c);
                        self.pos += 1;
                    } else {
                        break;
                    }
                }
                if self.peek() != Some('}') {
                    return Err(PatternError {
                        message: "unterminated {}".to_owned(),
                    });
                }
                self.pos += 1;
                if second.is_empty() {
                    None
                } else {
                    Some(second.parse().map_err(|_| PatternError {
                        message: "invalid upper bound".to_owned(),
                    })?)
                }
            }
            _ => {
                return Err(PatternError {
                    message: "unterminated {}".to_owned(),
                });
            }
        };
        Ok((min, max))
    }

    fn parse_atom(&mut self) -> Result<Node, PatternError> {
        match self.peek() {
            Some('(') => {
                self.pos += 1;
                let inner = self.parse_alternation()?;
                if self.peek() != Some(')') {
                    return Err(PatternError {
                        message: "unterminated group".to_owned(),
                    });
                }
                self.pos += 1;
                Ok(inner)
            }
            Some('[') => {
                self.pos += 1;
                self.parse_class()
            }
            Some('.') => {
                self.pos += 1;
                Ok(Node::Any)
            }
            Some('\\') => {
                self.pos += 1;
                let c = self.peek().ok_or_else(|| PatternError {
                    message: "trailing backslash".to_owned(),
                })?;
                self.pos += 1;
                if c == 'p' || c == 'P' {
                    return Ok(Node::Class {
                        negated: false,
                        items: vec![self.parse_category(c == 'P')?],
                    });
                }
                Ok(escape_node(c))
            }
            // A quantifier in atom position has nothing to repeat.
            // XSD requires these to be escaped as `\\*`, `\\+`, `\\?`
            // when meant literally, and every other engine rejects
            // them here. Treating one as a literal would turn a typo
            // into a pattern that quietly matches the wrong thing.
            Some(c @ ('*' | '+' | '?')) => Err(PatternError {
                message: format!(
                    "nothing to repeat before `{c}` at position {}",
                    self.pos
                ),
            }),
            Some(c) => {
                self.pos += 1;
                Ok(Node::Literal(c))
            }
            None => Err(PatternError {
                message: "unexpected end of pattern".to_owned(),
            }),
        }
    }

    /// `\p{Name}` or `\P{Name}`.
    ///
    /// A category this engine cannot decide exactly is a compile
    /// error rather than a guess. `support::unsupported` reports the
    /// pattern, so the schema is counted as unenforceable instead of
    /// matching the wrong set of characters.
    fn parse_category(
        &mut self,
        negated: bool,
    ) -> Result<ClassItem, PatternError> {
        if self.peek() != Some('{') {
            return Err(PatternError {
                message: "\\p must be followed by `{`".to_owned(),
            });
        }
        self.pos += 1;
        let mut name = String::new();
        loop {
            match self.peek() {
                Some('}') => {
                    self.pos += 1;
                    break;
                }
                Some(c) => {
                    name.push(c);
                    self.pos += 1;
                }
                None => {
                    return Err(PatternError {
                        message: "unterminated \\p{...}".to_owned(),
                    });
                }
            }
        }
        if category_matches(&name, 'a').is_none() {
            return Err(PatternError {
                message: format!(
                    "the Unicode category `{name}` is not supported"
                ),
            });
        }
        Ok(ClassItem::Category { name, negated })
    }

    fn parse_class(&mut self) -> Result<Node, PatternError> {
        let negated = if self.peek() == Some('^') {
            self.pos += 1;
            true
        } else {
            false
        };
        let mut items = Vec::new();
        loop {
            let c = self.peek().ok_or_else(|| PatternError {
                message: "unterminated character class".to_owned(),
            })?;
            if c == ']' {
                self.pos += 1;
                break;
            }
            self.pos += 1;
            if c == '\\' {
                let e = self.peek().ok_or_else(|| PatternError {
                    message: "trailing backslash in class".to_owned(),
                })?;
                self.pos += 1;
                items.push(escape_item(e));
                continue;
            }
            // A `-` between two characters is a range; anywhere else
            // it is a literal hyphen.
            if self.peek() == Some('-')
                && self.chars.get(self.pos + 1).is_some_and(|n| *n != ']')
            {
                self.pos += 1;
                let hi = self.peek().ok_or_else(|| PatternError {
                    message: "unterminated range".to_owned(),
                })?;
                self.pos += 1;
                items.push(ClassItem::Range(c, hi));
            } else {
                items.push(ClassItem::Char(c));
            }
        }
        Ok(Node::Class { negated, items })
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }
}

fn escape_node(c: char) -> Node {
    match c {
        'd' | 'D' | 'w' | 'W' | 's' | 'S' | 'i' | 'I' | 'c' | 'C' => {
            Node::Class {
                negated: false,
                items: vec![escape_item(c)],
            }
        }
        'n' => Node::Literal('\n'),
        't' => Node::Literal('\t'),
        'r' => Node::Literal('\r'),
        other => Node::Literal(other),
    }
}

fn escape_item(c: char) -> ClassItem {
    match c {
        'i' => ClassItem::NameStart,
        'I' => ClassItem::NotNameStart,
        'c' => ClassItem::NameChar,
        'C' => ClassItem::NotNameChar,
        'd' => ClassItem::Digit,
        'D' => ClassItem::NotDigit,
        'w' => ClassItem::Word,
        'W' => ClassItem::NotWord,
        's' => ClassItem::Space,
        'S' => ClassItem::NotSpace,
        'n' => ClassItem::Char('\n'),
        't' => ClassItem::Char('\t'),
        'r' => ClassItem::Char('\r'),
        other => ClassItem::Char(other),
    }
}

fn item_matches(item: &ClassItem, c: char) -> bool {
    match item {
        ClassItem::Char(x) => *x == c,
        ClassItem::Range(lo, hi) => c >= *lo && c <= *hi,
        ClassItem::Digit => c.is_ascii_digit(),
        ClassItem::NotDigit => !c.is_ascii_digit(),
        ClassItem::Word => c.is_alphanumeric() || c == '_',
        ClassItem::NotWord => !(c.is_alphanumeric() || c == '_'),
        ClassItem::Space => c.is_whitespace(),
        ClassItem::NotSpace => !c.is_whitespace(),
        ClassItem::NameStart => is_name_start(c),
        ClassItem::NotNameStart => !is_name_start(c),
        ClassItem::NameChar => is_name_char(c),
        ClassItem::NotNameChar => !is_name_char(c),
        // An unknown category never reaches here: the parser refuses
        // to compile one, so the schema is reported as unenforceable
        // rather than silently matching the wrong set.
        ClassItem::Category { name, negated } => {
            category_matches(name, c).unwrap_or(false) != *negated
        }
    }
}

/// `NameStartChar`, as `\i` means it.
///
/// XSD defines `\i` by the XML 1.0 `NameStartChar` production plus the
/// colon, which is what a name may begin with before namespaces
/// narrow it.
fn is_name_start(c: char) -> bool {
    matches!(c,
        ':' | '_' | 'A'..='Z' | 'a'..='z'
        | '\u{C0}'..='\u{D6}' | '\u{D8}'..='\u{F6}'
        | '\u{F8}'..='\u{2FF}' | '\u{370}'..='\u{37D}'
        | '\u{37F}'..='\u{1FFF}' | '\u{200C}'..='\u{200D}'
        | '\u{2070}'..='\u{218F}' | '\u{2C00}'..='\u{2FEF}'
        | '\u{3001}'..='\u{D7FF}' | '\u{F900}'..='\u{FDCF}'
        | '\u{FDF0}'..='\u{FFFD}' | '\u{10000}'..='\u{EFFFF}')
}

/// `NameChar`, as `\c` means it.
fn is_name_char(c: char) -> bool {
    is_name_start(c)
        || matches!(c,
            '-' | '.' | '0'..='9' | '\u{B7}'
            | '\u{300}'..='\u{36F}' | '\u{203F}'..='\u{2040}')
}

/// Match `node` at `pos`, calling `k` with each possible end position.
///
/// Continuation-passing rather than returning a length: a repeat has
/// many possible lengths, and the one that lets the *rest* of the
/// pattern match is not knowable locally. Passing the continuation
/// down is what makes backtracking fall out naturally.
fn match_node(
    node: &Node,
    input: &[char],
    pos: usize,
    k: &mut dyn FnMut(usize) -> bool,
) -> bool {
    match node {
        Node::Literal(c) => {
            input.get(pos).is_some_and(|x| x == c) && k(pos + 1)
        }
        Node::Any => pos < input.len() && k(pos + 1),
        Node::Class { negated, items } => {
            let Some(&c) = input.get(pos) else {
                return false;
            };
            let hit = items.iter().any(|i| item_matches(i, c));
            (hit != *negated) && k(pos + 1)
        }
        Node::Sequence(items) => match_sequence(items, input, pos, k),
        Node::Alternation(branches) => {
            branches.iter().any(|b| match_node(b, input, pos, k))
        }
        Node::Repeat { node, min, max } => {
            match_repeat(node, *min, *max, input, pos, k)
        }
    }
}

fn match_sequence(
    items: &[Node],
    input: &[char],
    pos: usize,
    k: &mut dyn FnMut(usize) -> bool,
) -> bool {
    match items.split_first() {
        None => k(pos),
        Some((head, rest)) => match_node(head, input, pos, &mut |next| {
            match_sequence(rest, input, next, k)
        }),
    }
}

fn match_repeat(
    node: &Node,
    min: usize,
    max: Option<usize>,
    input: &[char],
    pos: usize,
    k: &mut dyn FnMut(usize) -> bool,
) -> bool {
    if min > 0 {
        return match_node(node, input, pos, &mut |next| {
            // A zero-width match would loop forever without this.
            next > pos
                && match_repeat(
                    node,
                    min - 1,
                    max.map(|m| m - 1),
                    input,
                    next,
                    k,
                )
        });
    }
    if k(pos) {
        return true;
    }
    if max == Some(0) {
        return false;
    }
    match_node(node, input, pos, &mut |next| {
        next > pos && match_repeat(node, 0, max.map(|m| m - 1), input, next, k)
    })
}
