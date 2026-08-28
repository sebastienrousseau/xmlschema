#![no_main]
//! Validating an arbitrary document against an arbitrary schema must
//! never panic.
//!
//! Parsing a schema is one surface; *using* it is another, and the
//! second is where a mismatch between two untrusted inputs lands. A
//! service that validates user documents against user schemas
//! exercises exactly this pair.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let text = match core::str::from_utf8(data) {
        Ok(s) => s,
        Err(e) => match core::str::from_utf8(&data[..e.valid_up_to()]) {
            Ok(s) => s,
            Err(_) => return,
        },
    };

    // Split the input into a schema and a document. A single fuzzed
    // buffer has to supply both, or the pair is never exercised
    // together -- and a corpus of schemas alone would never reach
    // `validate` at all.
    let (xsd, xml) = match text.split_once("\u{0}") {
        Some(pair) => pair,
        // No separator: validate the input against itself. A document
        // that is its own schema is unusual and perfectly legal to
        // ask about.
        None => (text, text),
    };

    let Ok(schema) = xmlschema::parse_schema(xsd) else {
        return;
    };
    let Ok(doc) = oxml::parse(xml) else {
        return;
    };
    let report = xmlschema::validate(&doc, &schema);
    // Every violation must be readable: a report nobody can print is
    // no better than a panic to a caller trying to explain a failure.
    for v in report.violations.iter().take(64) {
        let _ = v.path.len();
        let _ = v.message.len();
    }
});
