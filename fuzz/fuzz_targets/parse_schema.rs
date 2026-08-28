#![no_main]
//! Arbitrary bytes must never panic the schema parser.
//!
//! A schema is an input the caller did not write. `xmlschema` is
//! reached through `oxml-cli`, `oxml-mcp` and any service that
//! validates a document against a schema it was handed, so the
//! contract is the same one `oxml` holds: any input at all produces
//! `Ok` or `Err`, and nothing else.
//!
//! This crate had no fuzz targets while parsing 6,790 lines of
//! untrusted XSD. `oxml` has six, and each found real defects.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Only valid UTF-8 reaches the parser, which takes `&str`.
    // Recover the longest valid prefix rather than discarding most of
    // the corpus.
    let text = match core::str::from_utf8(data) {
        Ok(s) => s,
        Err(e) => match core::str::from_utf8(&data[..e.valid_up_to()]) {
            Ok(s) => s,
            Err(_) => return,
        },
    };

    if let Ok(schema) = xmlschema::parse_schema(text) {
        // A schema that parsed must be inspectable without panicking:
        // a malformed-but-accepted one must not hide a broken
        // invariant behind a lazily-read field.
        let _ = schema.elements.len();
        let _ = schema.named_simple_types.len();
        let _ = schema.named_complex_types.len();
        let _ = schema.target_namespace.as_deref();
        for name in schema.elements.keys().take(64) {
            let _ = schema.elements.get(name);
        }
    }
});
