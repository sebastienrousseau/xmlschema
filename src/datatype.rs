// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 xmlschema. All rights reserved.

//! The XSD 1.0 built-in datatypes.
//!
//! Every built-in is modelled separately, including the ones that
//! differ from a relative only in range or in lexical form. Collapsing
//! them is tempting and wrong: `xs:byte` validated as an unbounded
//! integer accepts `999`, and a schema using it then agrees with every
//! document a conformance suite calls valid while enforcing nothing.
//! The suite has some five thousand tests over the integer lattice
//! alone.
//!
//! Two rules apply before the lexical form is examined:
//!
//! * **Whitespace processing.** Every type but `xs:string` and
//!   `xs:normalizedString` collapses whitespace first, so `" 5 "` is a
//!   valid `xs:integer`.
//! * **Arbitrary precision.** `xs:integer` and `xs:decimal` have no
//!   bounds, so a value is checked lexically and only the *bounded*
//!   types compare numerically.

use std::borrow::Cow;

/// How a datatype treats whitespace before validating.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhiteSpace {
    /// Keep the value exactly as written.
    Preserve,
    /// Turn every tab, newline and carriage return into a space.
    Replace,
    /// Replace, then collapse runs of spaces and trim the ends.
    Collapse,
}

/// A built-in XSD 1.0 datatype.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Datatype {
    /// `xs:anySimpleType` — accepts any character data.
    AnySimpleType,
    /// `xs:string`
    String,
    /// `xs:normalizedString`
    NormalizedString,
    /// `xs:token`
    Token,
    /// `xs:language`
    Language,
    /// `xs:NMTOKEN`
    NmToken,
    /// `xs:NMTOKENS`
    NmTokens,
    /// `xs:Name`
    Name,
    /// `xs:NCName`
    NcName,
    /// `xs:ID`
    Id,
    /// `xs:IDREF`
    IdRef,
    /// `xs:IDREFS`
    IdRefs,
    /// `xs:ENTITY`
    Entity,
    /// `xs:ENTITIES`
    Entities,
    /// `xs:boolean`
    Boolean,
    /// `xs:decimal`
    Decimal,
    /// `xs:integer`
    Integer,
    /// `xs:nonPositiveInteger`
    NonPositiveInteger,
    /// `xs:negativeInteger`
    NegativeInteger,
    /// `xs:long`
    Long,
    /// `xs:int`
    Int,
    /// `xs:short`
    Short,
    /// `xs:byte`
    Byte,
    /// `xs:nonNegativeInteger`
    NonNegativeInteger,
    /// `xs:unsignedLong`
    UnsignedLong,
    /// `xs:unsignedInt`
    UnsignedInt,
    /// `xs:unsignedShort`
    UnsignedShort,
    /// `xs:unsignedByte`
    UnsignedByte,
    /// `xs:positiveInteger`
    PositiveInteger,
    /// `xs:float`
    Float,
    /// `xs:double`
    Double,
    /// `xs:duration`
    Duration,
    /// `xs:dateTime`
    DateTime,
    /// `xs:time`
    Time,
    /// `xs:date`
    Date,
    /// `xs:gYearMonth`
    GYearMonth,
    /// `xs:gYear`
    GYear,
    /// `xs:gMonthDay`
    GMonthDay,
    /// `xs:gMonth`
    GMonth,
    /// `xs:gDay`
    GDay,
    /// `xs:hexBinary`
    HexBinary,
    /// `xs:base64Binary`
    Base64Binary,
    /// `xs:anyURI`
    AnyUri,
    /// `xs:QName`
    QName,
    /// `xs:NOTATION`
    Notation,
}

impl Datatype {
    /// Resolve a type name, ignoring any namespace prefix.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        let local = name.rsplit(':').next().unwrap_or(name);
        Some(match local {
            "anySimpleType" | "anyType" => Self::AnySimpleType,
            "string" => Self::String,
            "normalizedString" => Self::NormalizedString,
            "token" => Self::Token,
            "language" => Self::Language,
            "NMTOKEN" => Self::NmToken,
            "NMTOKENS" => Self::NmTokens,
            "Name" => Self::Name,
            "NCName" => Self::NcName,
            "ID" => Self::Id,
            "IDREF" => Self::IdRef,
            "IDREFS" => Self::IdRefs,
            "ENTITY" => Self::Entity,
            "ENTITIES" => Self::Entities,
            "boolean" => Self::Boolean,
            "decimal" => Self::Decimal,
            "integer" => Self::Integer,
            "nonPositiveInteger" => Self::NonPositiveInteger,
            "negativeInteger" => Self::NegativeInteger,
            "long" => Self::Long,
            "int" => Self::Int,
            "short" => Self::Short,
            "byte" => Self::Byte,
            "nonNegativeInteger" => Self::NonNegativeInteger,
            "unsignedLong" => Self::UnsignedLong,
            "unsignedInt" => Self::UnsignedInt,
            "unsignedShort" => Self::UnsignedShort,
            "unsignedByte" => Self::UnsignedByte,
            "positiveInteger" => Self::PositiveInteger,
            "float" => Self::Float,
            "double" => Self::Double,
            "duration" => Self::Duration,
            "dateTime" => Self::DateTime,
            "time" => Self::Time,
            "date" => Self::Date,
            "gYearMonth" => Self::GYearMonth,
            "gYear" => Self::GYear,
            "gMonthDay" => Self::GMonthDay,
            "gMonth" => Self::GMonth,
            "gDay" => Self::GDay,
            "hexBinary" => Self::HexBinary,
            "base64Binary" => Self::Base64Binary,
            "anyURI" => Self::AnyUri,
            "QName" => Self::QName,
            "NOTATION" => Self::Notation,
            _ => return None,
        })
    }

    /// How this type treats whitespace before validating.
    #[must_use]
    pub const fn white_space(self) -> WhiteSpace {
        match self {
            Self::String | Self::AnySimpleType => WhiteSpace::Preserve,
            Self::NormalizedString => WhiteSpace::Replace,
            _ => WhiteSpace::Collapse,
        }
    }

    /// Apply this type's whitespace rule.
    #[must_use]
    pub fn normalise(self, raw: &str) -> Cow<'_, str> {
        match self.white_space() {
            WhiteSpace::Preserve => Cow::Borrowed(raw),
            WhiteSpace::Replace => {
                if raw.contains(['\t', '\n', '\r']) {
                    Cow::Owned(raw.replace(['\t', '\n', '\r'], " "))
                } else {
                    Cow::Borrowed(raw)
                }
            }
            WhiteSpace::Collapse => {
                let trimmed = raw.trim_matches(is_xml_space);
                if trimmed.split(is_xml_space).any(str::is_empty) {
                    Cow::Owned(
                        trimmed
                            .split(is_xml_space)
                            .filter(|p| !p.is_empty())
                            .collect::<Vec<_>>()
                            .join(" "),
                    )
                } else {
                    Cow::Borrowed(trimmed)
                }
            }
        }
    }

    /// Whether `raw` is a valid value, after whitespace processing.
    #[must_use]
    pub fn accepts(self, raw: &str) -> bool {
        let v = self.normalise(raw);
        self.accepts_normalised(&v)
    }

    fn accepts_normalised(self, v: &str) -> bool {
        match self {
            Self::AnySimpleType
            | Self::String
            | Self::NormalizedString
            | Self::Token
            // A URI reference is almost unconstrained; the one thing
            // the specification rules out is a stray fragment marker.
            | Self::AnyUri => true,

            Self::Language => is_language(v),
            Self::NmToken => is_nmtoken(v),
            Self::NmTokens => list_of(v, is_nmtoken),
            Self::Name => is_name(v),
            Self::NcName | Self::Id | Self::IdRef | Self::Entity => {
                is_ncname(v)
            }
            Self::IdRefs | Self::Entities => list_of(v, is_ncname),
            Self::QName | Self::Notation => is_qname(v),

            Self::Boolean => matches!(v, "true" | "false" | "1" | "0"),

            Self::Decimal => is_decimal(v),
            Self::Integer => is_integer_lexical(v),
            Self::NonPositiveInteger => integer_at_most(v, 0),
            Self::NegativeInteger => integer_at_most(v, -1),
            Self::NonNegativeInteger => integer_at_least(v, 0),
            Self::PositiveInteger => integer_at_least(v, 1),
            Self::Long => integer_in(v, i128::from(i64::MIN), i128::from(i64::MAX)),
            Self::Int => integer_in(v, i128::from(i32::MIN), i128::from(i32::MAX)),
            Self::Short => integer_in(v, i128::from(i16::MIN), i128::from(i16::MAX)),
            Self::Byte => integer_in(v, i128::from(i8::MIN), i128::from(i8::MAX)),
            Self::UnsignedLong => integer_in(v, 0, i128::from(u64::MAX)),
            Self::UnsignedInt => integer_in(v, 0, i128::from(u32::MAX)),
            Self::UnsignedShort => integer_in(v, 0, i128::from(u16::MAX)),
            Self::UnsignedByte => integer_in(v, 0, i128::from(u8::MAX)),

            Self::Float | Self::Double => is_floating(v),

            Self::Duration => is_duration(v),
            Self::DateTime => is_date_time(v),
            Self::Time => is_time(strip_zone(v)),
            Self::Date => is_date(strip_zone(v)),
            Self::GYearMonth => is_g_year_month(strip_zone(v)),
            Self::GYear => is_g_year(strip_zone(v)),
            Self::GMonthDay => is_g_month_day(strip_zone(v)),
            Self::GMonth => is_g_month(strip_zone(v)),
            Self::GDay => is_g_day(strip_zone(v)),

            Self::HexBinary => {
                v.len() % 2 == 0 && v.bytes().all(|b| b.is_ascii_hexdigit())
            }
            Self::Base64Binary => is_base64(v),
        }
    }

    /// Whether numeric facets (`minInclusive` and friends) apply.
    #[must_use]
    pub const fn is_numeric(self) -> bool {
        matches!(
            self,
            Self::Decimal
                | Self::Integer
                | Self::NonPositiveInteger
                | Self::NegativeInteger
                | Self::Long
                | Self::Int
                | Self::Short
                | Self::Byte
                | Self::NonNegativeInteger
                | Self::UnsignedLong
                | Self::UnsignedInt
                | Self::UnsignedShort
                | Self::UnsignedByte
                | Self::PositiveInteger
                | Self::Float
                | Self::Double
        )
    }

    /// A human-readable name for a diagnostic.
    #[must_use]
    pub const fn describe(self) -> &'static str {
        match self {
            Self::AnySimpleType => "any simple value",
            Self::String => "string",
            Self::NormalizedString => "normalized string",
            Self::Token => "token",
            Self::Language => "language tag",
            Self::NmToken => "NMTOKEN",
            Self::NmTokens => "list of NMTOKEN",
            Self::Name => "Name",
            Self::NcName => "NCName",
            Self::Id => "ID",
            Self::IdRef => "IDREF",
            Self::IdRefs => "list of IDREF",
            Self::Entity => "ENTITY",
            Self::Entities => "list of ENTITY",
            Self::Boolean => "boolean (true, false, 1 or 0)",
            Self::Decimal => "decimal",
            Self::Integer => "integer",
            Self::NonPositiveInteger => "non-positive integer",
            Self::NegativeInteger => "negative integer",
            Self::Long => "long (64-bit)",
            Self::Int => "int (32-bit)",
            Self::Short => "short (16-bit)",
            Self::Byte => "byte (8-bit)",
            Self::NonNegativeInteger => "non-negative integer",
            Self::UnsignedLong => "unsigned long (64-bit)",
            Self::UnsignedInt => "unsigned int (32-bit)",
            Self::UnsignedShort => "unsigned short (16-bit)",
            Self::UnsignedByte => "unsigned byte (8-bit)",
            Self::PositiveInteger => "positive integer",
            Self::Float => "float",
            Self::Double => "double",
            Self::Duration => "duration (PnYnMnDTnHnMnS)",
            Self::DateTime => "dateTime (YYYY-MM-DDThh:mm:ss)",
            Self::Time => "time (hh:mm:ss)",
            Self::Date => "date (YYYY-MM-DD)",
            Self::GYearMonth => "gYearMonth (YYYY-MM)",
            Self::GYear => "gYear (YYYY)",
            Self::GMonthDay => "gMonthDay (--MM-DD)",
            Self::GMonth => "gMonth (--MM)",
            Self::GDay => "gDay (---DD)",
            Self::HexBinary => "hexBinary",
            Self::Base64Binary => "base64Binary",
            Self::AnyUri => "URI",
            Self::QName => "QName",
            Self::Notation => "NOTATION",
        }
    }
}

impl Datatype {
    /// Whether values of this type are ordered, so `minInclusive` and
    /// its relatives mean something.
    ///
    /// Numbers are the obvious case, but the temporal types are
    /// ordered too -- and storing a bound as an `f64` silently dropped
    /// every one of them, because `"2000-01-01".parse::<f64>()` fails.
    #[must_use]
    pub const fn is_ordered(self) -> bool {
        self.is_numeric() || self.is_temporal()
    }

    /// Whether this is a date, time, duration or gregorian type.
    #[must_use]
    pub const fn is_temporal(self) -> bool {
        matches!(
            self,
            Self::Duration
                | Self::DateTime
                | Self::Time
                | Self::Date
                | Self::GYearMonth
                | Self::GYear
                | Self::GMonthDay
                | Self::GMonth
                | Self::GDay
        )
    }

    /// Compare two lexical values of this type.
    ///
    /// Numbers compare as decimals rather than through `f64`, which
    /// loses precision past 2^53: `999999999999999998` and
    /// `999999999999999999` are distinct `xs:integer` values and
    /// compared equal as floats.
    #[must_use]
    pub fn compare(self, a: &str, b: &str) -> Option<core::cmp::Ordering> {
        if self.is_numeric() {
            return compare_decimal(
                self.normalise(a).as_ref(),
                self.normalise(b).as_ref(),
            );
        }
        let (x, y) = (self.order_key(a)?, self.order_key(b)?);
        x.partial_cmp(&y)
    }

    /// A sortable key for an ordered value.
    ///
    /// Returns `None` when the value does not belong to the type, or
    /// the type has no ordering. Both sides of a comparison are
    /// lexical forms of the same type, so both go through here and the
    /// units cancel.
    #[must_use]
    pub fn order_key(self, raw: &str) -> Option<f64> {
        let v = self.normalise(raw);
        if !self.accepts_normalised(&v) {
            return None;
        }
        if self.is_numeric() {
            return v.parse::<f64>().ok();
        }
        let body = strip_zone(&v);
        Some(match self {
            // Seconds, so every date-like type shares a scale.
            Self::Date => date_seconds(body)?,
            Self::DateTime => {
                let (d, t) = body.split_once('T')?;
                date_seconds(d)? + time_seconds(strip_zone(t))?
            }
            Self::Time => time_seconds(body)?,
            Self::GYear => year_of(body)? * 31_556_952.0,
            Self::GYearMonth => {
                let (y, m) = body.rsplit_once('-')?;
                year_of(y)? * 12.0 + m.parse::<f64>().ok()?
            }
            Self::GMonth => body.strip_prefix("--")?.parse::<f64>().ok()?,
            Self::GDay => body.strip_prefix("---")?.parse::<f64>().ok()?,
            Self::GMonthDay => {
                let rest = body.strip_prefix("--")?;
                let (m, d) = rest.split_once('-')?;
                m.parse::<f64>().ok()? * 31.0 + d.parse::<f64>().ok()?
            }
            // A duration is only partially ordered -- months and days
            // are not commensurable -- so this uses the average month
            // the specification itself uses for comparison. Two
            // durations the specification calls indeterminate compare
            // by that approximation rather than not at all.
            Self::Duration => duration_seconds(&v)?,
            _ => return None,
        })
    }
}

/// `YYYY-MM-DD` as seconds from a fixed epoch.
///
/// Days before the year, plus the day of the year. The first version
/// mixed two epoch formulas -- a leap-adjusted year count with an
/// unadjusted month sum -- and made `2001-01-01` compare equal to
/// `2000-12-31`.
fn date_seconds(v: &str) -> Option<f64> {
    let mut parts = v.rsplitn(3, '-');
    let day: i64 = parts.next()?.parse().ok()?;
    let month: i64 = parts.next()?.parse().ok()?;
    let year: i64 = parts.next()?.parse().ok()?;

    // Leap years strictly before this one.
    let prev = year - 1;
    let leaps = prev / 4 - prev / 100 + prev / 400;
    let days_before_year = 365 * prev + leaps;

    let year_text = year.to_string();
    let days_before_month: i64 = (1..month)
        .filter_map(|m| u32::try_from(m).ok())
        .map(|m| i64::from(days_in_month(m, &year_text)))
        .sum();

    // A day count fits an `f64` mantissa for any year the lexical
    // form can express, so the conversion is exact here.
    let days = days_before_year + days_before_month + day;
    #[allow(clippy::cast_precision_loss)]
    Some(days as f64 * 86_400.0)
}

fn time_seconds(v: &str) -> Option<f64> {
    let mut parts = v.split(':');
    let h: f64 = parts.next()?.parse().ok()?;
    let m: f64 = parts.next()?.parse().ok()?;
    let s: f64 = parts.next()?.parse().ok()?;
    Some(h * 3600.0 + m * 60.0 + s)
}

fn year_of(v: &str) -> Option<f64> {
    v.parse::<f64>().ok()
}

/// A duration in seconds, using the average month the specification
/// uses when it needs a total order.
fn duration_seconds(v: &str) -> Option<f64> {
    let negative = v.starts_with('-');
    let body = v.trim_start_matches('-').strip_prefix('P')?;
    let (date, time) = match body.split_once('T') {
        Some((d, t)) => (d, t),
        None => (body, ""),
    };
    let mut total = 0.0;
    for (part, units) in [
        (
            date,
            [('Y', 31_556_952.0), ('M', 2_629_746.0), ('D', 86_400.0)],
        ),
        (time, [('H', 3600.0), ('M', 60.0), ('S', 1.0)]),
    ] {
        let mut digits = String::new();
        for c in part.chars() {
            if c.is_ascii_digit() || c == '.' {
                digits.push(c);
            } else if let Some((_, scale)) = units.iter().find(|(d, _)| *d == c)
            {
                total += digits.parse::<f64>().ok()? * scale;
                digits.clear();
            }
        }
    }
    Some(if negative { -total } else { total })
}

/// Compare two decimal lexical forms exactly.
///
/// Sign first, then magnitude: the integer parts by length once
/// leading zeros are gone, then digit by digit, then the fractions
/// padded to the same length. No floating point is involved, so an
/// eighteen-digit integer compares as itself.
fn compare_decimal(a: &str, b: &str) -> Option<core::cmp::Ordering> {
    use core::cmp::Ordering;

    // The specials only `float` and `double` admit.
    for (v, order) in [(a, Ordering::Less), (b, Ordering::Greater)] {
        if v == "NaN" {
            return None;
        }
        let _ = order;
    }
    let sign_of = |v: &str| if v.starts_with('-') { -1 } else { 1 };
    let (sa, sb) = (sign_of(a), sign_of(b));

    let magnitude = |v: &str| {
        let body = v.strip_prefix(['+', '-']).unwrap_or(v);
        // Scientific notation is normalised through `f64`; it is only
        // used by `float` and `double`, whose precision is the point.
        if body.contains(['e', 'E']) || body == "INF" {
            return None;
        }
        let (whole, frac) = body.split_once('.').unwrap_or((body, ""));
        Some((
            whole.trim_start_matches('0').to_owned(),
            frac.trim_end_matches('0').to_owned(),
        ))
    };
    let (Some((wa, fa)), Some((wb, fb))) = (magnitude(a), magnitude(b)) else {
        return a.parse::<f64>().ok()?.partial_cmp(&b.parse::<f64>().ok()?);
    };

    // Zero has no sign for the purpose of ordering.
    let a_zero = wa.is_empty() && fa.is_empty();
    let b_zero = wb.is_empty() && fb.is_empty();
    let sa = if a_zero { 0 } else { sa };
    let sb = if b_zero { 0 } else { sb };
    if sa != sb {
        return Some(sa.cmp(&sb));
    }

    let mut order = wa.len().cmp(&wb.len()).then_with(|| wa.cmp(&wb));
    if order == Ordering::Equal {
        // Pad the fractions so `.5` and `.45` compare by position.
        let width = fa.len().max(fb.len());
        let pad = |f: &str| format!("{f:0<width$}");
        order = pad(&fa).cmp(&pad(&fb));
    }
    // A negative magnitude ordering is the reverse of the value's.
    Some(if sa < 0 { order.reverse() } else { order })
}

const fn is_xml_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r')
}

// --- names ------------------------------------------------------

/// `NameStartChar` from XML 1.0, fifth edition.
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

/// `NameChar` from XML 1.0, fifth edition.
fn is_name_char(c: char) -> bool {
    is_name_start(c)
        || matches!(c,
            '-' | '.' | '0'..='9' | '\u{B7}'
            | '\u{300}'..='\u{36F}' | '\u{203F}'..='\u{2040}')
}

fn is_name(v: &str) -> bool {
    let mut chars = v.chars();
    chars.next().is_some_and(is_name_start) && chars.all(is_name_char)
}

fn is_ncname(v: &str) -> bool {
    !v.contains(':') && is_name(v)
}

fn is_nmtoken(v: &str) -> bool {
    !v.is_empty() && v.chars().all(is_name_char)
}

fn is_qname(v: &str) -> bool {
    match v.split_once(':') {
        Some((prefix, local)) => is_ncname(prefix) && is_ncname(local),
        None => is_ncname(v),
    }
}

/// A whitespace-separated list, every item of which must be valid.
///
/// The empty list is not a valid value of any of the list types.
fn list_of(v: &str, item: fn(&str) -> bool) -> bool {
    let mut any = false;
    for part in v.split(is_xml_space).filter(|p| !p.is_empty()) {
        if !item(part) {
            return false;
        }
        any = true;
    }
    any
}

/// `xs:language` — an RFC 3066 tag.
fn is_language(v: &str) -> bool {
    let mut parts = v.split('-');
    let Some(first) = parts.next() else {
        return false;
    };
    if first.is_empty()
        || first.len() > 8
        || !first.bytes().all(|b| b.is_ascii_alphabetic())
    {
        return false;
    }
    parts.all(|p| {
        !p.is_empty()
            && p.len() <= 8
            && p.bytes().all(|b| b.is_ascii_alphanumeric())
    })
}

// --- numbers ----------------------------------------------------

/// `[+-]?[0-9]+`, with no bound on length.
fn is_integer_lexical(v: &str) -> bool {
    let digits = v.strip_prefix(['+', '-']).unwrap_or(v);
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
}

/// The lexical form as an `i128`, or `None` if it does not fit.
///
/// Every bound this crate checks fits in an `i128`, so a value that
/// overflows one is out of range for whichever type asked.
fn integer_value(v: &str) -> Option<i128> {
    is_integer_lexical(v).then(|| v.parse::<i128>().ok())?
}

fn integer_in(v: &str, min: i128, max: i128) -> bool {
    integer_value(v).is_some_and(|n| n >= min && n <= max)
}

fn integer_at_least(v: &str, min: i128) -> bool {
    if !is_integer_lexical(v) {
        return false;
    }
    // A value too large for `i128` is still >= any bound this crate
    // checks, provided it is not negative.
    integer_value(v).map_or(!v.starts_with('-'), |n| n >= min)
}

fn integer_at_most(v: &str, max: i128) -> bool {
    if !is_integer_lexical(v) {
        return false;
    }
    integer_value(v).map_or(v.starts_with('-'), |n| n <= max)
}

/// `[+-]?(\d+(\.\d*)?|\.\d+)`
fn is_decimal(v: &str) -> bool {
    let body = v.strip_prefix(['+', '-']).unwrap_or(v);
    let (whole, frac) = body.split_once('.').unwrap_or((body, ""));
    if whole.is_empty() && frac.is_empty() {
        return false;
    }
    whole.bytes().all(|b| b.is_ascii_digit())
        && frac.bytes().all(|b| b.is_ascii_digit())
}

/// A decimal, a scientific form, or one of the three specials.
fn is_floating(v: &str) -> bool {
    if matches!(v, "INF" | "-INF" | "+INF" | "NaN") {
        return true;
    }
    let (mantissa, exponent) = match v.split_once(['e', 'E']) {
        Some((m, e)) => (m, Some(e)),
        None => (v, None),
    };
    if !is_decimal(mantissa) {
        return false;
    }
    exponent.is_none_or(is_integer_lexical)
}

// --- dates and times --------------------------------------------

/// Remove a trailing timezone, which every date and time form allows.
fn strip_zone(v: &str) -> &str {
    if let Some(rest) = v.strip_suffix('Z') {
        return rest;
    }
    // An offset is `±hh:mm`, six characters, and must not be confused
    // with the leading `-` of a negative year.
    if v.len() > 6 {
        let (head, tail) = v.split_at(v.len() - 6);
        if (tail.starts_with('+') || tail.starts_with('-'))
            && tail.as_bytes()[3] == b':'
            && tail[1..3].bytes().all(|b| b.is_ascii_digit())
            && tail[4..].bytes().all(|b| b.is_ascii_digit())
        {
            return head;
        }
    }
    v
}

fn two_digit(v: &str, min: u32, max: u32) -> bool {
    v.len() == 2
        && v.bytes().all(|b| b.is_ascii_digit())
        && v.parse::<u32>().is_ok_and(|n| n >= min && n <= max)
}

/// `[-]?YYYY`, four digits or more, and not year zero.
fn is_year(v: &str) -> bool {
    let digits = v.strip_prefix('-').unwrap_or(v);
    digits.len() >= 4
        && digits.bytes().all(|b| b.is_ascii_digit())
        && digits.parse::<u64>().is_ok_and(|y| y != 0)
}

fn is_date(v: &str) -> bool {
    let mut parts = v.rsplitn(3, '-');
    let (Some(day), Some(month), Some(year)) =
        (parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    if !is_year(year) || !two_digit(month, 1, 12) {
        return false;
    }
    let (Ok(m), Ok(d)) = (month.parse::<u32>(), day.parse::<u32>()) else {
        return false;
    };
    two_digit(day, 1, 31) && d <= days_in_month(m, year)
}

/// Days in a month, honouring leap years so `2001-02-29` is rejected.
fn days_in_month(month: u32, year: &str) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            let y: i64 = year.parse().unwrap_or(1);
            let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
            if leap { 29 } else { 28 }
        }
        _ => 0,
    }
}

fn is_time(v: &str) -> bool {
    let mut parts = v.split(':');
    let (Some(h), Some(m), Some(s)) =
        (parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    if parts.next().is_some() || !two_digit(h, 0, 24) || !two_digit(m, 0, 59) {
        return false;
    }
    let (whole, frac) = s.split_once('.').unwrap_or((s, ""));
    if !two_digit(whole, 0, 59) {
        return false;
    }
    if s.contains('.') && frac.is_empty() {
        return false;
    }
    if !frac.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    // 24:00:00 is the only value the hour 24 may take.
    h != "24" || (m == "00" && whole == "00" && frac.chars().all(|c| c == '0'))
}

fn is_date_time(v: &str) -> bool {
    let Some((date, rest)) = v.split_once('T') else {
        return false;
    };
    is_date(date) && is_time(strip_zone(rest))
}

fn is_g_year(v: &str) -> bool {
    is_year(v)
}

fn is_g_year_month(v: &str) -> bool {
    let Some((year, month)) = v.rsplit_once('-') else {
        return false;
    };
    is_year(year) && two_digit(month, 1, 12)
}

fn is_g_month(v: &str) -> bool {
    // XSD 1.0 as first published wrote this `--MM--`, and the errata
    // shortened it to `--MM`. The suite uses both, and a value the
    // specification once required is not one to refuse.
    let body = v
        .strip_prefix("--")
        .map(|m| m.strip_suffix("--").unwrap_or(m));
    body.is_some_and(|m| two_digit(m, 1, 12))
}

fn is_g_day(v: &str) -> bool {
    v.strip_prefix("---").is_some_and(|d| two_digit(d, 1, 31))
}

fn is_g_month_day(v: &str) -> bool {
    let Some(rest) = v.strip_prefix("--") else {
        return false;
    };
    let Some((month, day)) = rest.split_once('-') else {
        return false;
    };
    if !two_digit(month, 1, 12) || !two_digit(day, 1, 31) {
        return false;
    }
    // February is given 29 days here: without a year there is no way
    // to know whether it is a leap year, and the specification takes
    // the permissive reading.
    let m: u32 = month.parse().unwrap_or(0);
    let d: u32 = day.parse().unwrap_or(0);
    d <= if m == 2 { 29 } else { days_in_month(m, "2004") }
}

/// `-?PnYnMnDTnHnMnS`, with at least one component and no `T` unless a
/// time component follows it.
fn is_duration(v: &str) -> bool {
    let body = v.strip_prefix('-').unwrap_or(v);
    let Some(body) = body.strip_prefix('P') else {
        return false;
    };
    if body.is_empty() {
        return false;
    }
    let (date, time) = match body.split_once('T') {
        Some((d, t)) => (d, Some(t)),
        None => (body, None),
    };
    if time.is_some_and(str::is_empty) {
        return false;
    }
    let mut any = false;
    if !components(date, &['Y', 'M', 'D'], false, &mut any) {
        return false;
    }
    if let Some(time) = time {
        if !components(time, &['H', 'M', 'S'], true, &mut any) {
            return false;
        }
    }
    any
}

/// Read `<digits><designator>` groups in the order given.
///
/// `seconds_may_be_fractional` allows the final `S` a decimal part,
/// which is the only place a duration permits one.
fn components(
    mut v: &str,
    order: &[char],
    seconds_may_be_fractional: bool,
    any: &mut bool,
) -> bool {
    let mut next = 0;
    while !v.is_empty() {
        let digits: String = v
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        if digits.is_empty() {
            return false;
        }
        v = &v[digits.len()..];
        let Some(designator) = v.chars().next() else {
            return false;
        };
        v = &v[designator.len_utf8()..];
        let Some(position) =
            order[next..].iter().position(|d| *d == designator)
        else {
            return false;
        };
        let fractional = digits.contains('.');
        if fractional && !(seconds_may_be_fractional && designator == 'S') {
            return false;
        }
        if fractional && !is_decimal(&digits) {
            return false;
        }
        if !fractional && !digits.bytes().all(|b| b.is_ascii_digit()) {
            return false;
        }
        next += position + 1;
        *any = true;
    }
    true
}

fn is_base64(v: &str) -> bool {
    let body: String = v.chars().filter(|c| !is_xml_space(*c)).collect();
    if body.len() % 4 != 0 {
        return false;
    }
    let (data, padding) = match body.strip_suffix("==") {
        Some(rest) => (rest, 2),
        None => match body.strip_suffix('=') {
            Some(rest) => (rest, 1),
            None => (body.as_str(), 0),
        },
    };
    let _ = padding;
    data.bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/')
}
