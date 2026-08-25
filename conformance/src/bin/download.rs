// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 xmlschema. All rights reserved.

//! Fetch and verify the W3C XML Schema Test Suite.
//!
//! Uses `curl` and `tar` rather than pulling in an HTTP client and a
//! decompressor, which would be a large dependency surface for a
//! development-only task.

use std::path::PathBuf;
use std::process::Command;

use xmlschema_conformance::{SUITE_SHA256, SUITE_URL, sha256};

fn main() -> Result<(), String> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data");
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let tarball = root.join("xsts.tar.gz");

    println!("downloading {SUITE_URL}");
    // The User-Agent is load-bearing, not cargo-culting. www.w3.org is
    // behind Cloudflare: a request without a browser UA gets an HTML
    // challenge page with HTTP 403 or 200. Piping that into `tar`
    // yields an empty directory, and a conformance job then reports
    // success having run zero tests.
    let status = Command::new("curl")
        .args([
            "--fail",
            "--location",
            "--silent",
            "--show-error",
            "--user-agent",
            "Mozilla/5.0 (compatible; xmlschema-conformance/0.0; \
             +https://github.com/sebastienrousseau/xmlschema)",
            "--output",
        ])
        .arg(&tarball)
        .arg(SUITE_URL)
        .status()
        .map_err(|e| format!("could not run curl: {e}"))?;
    if !status.success() {
        return Err(format!("curl failed: {status}"));
    }

    let bytes = std::fs::read(&tarball).map_err(|e| e.to_string())?;
    let digest = sha256::sha256(&bytes);
    if digest != SUITE_SHA256 {
        return Err(format!(
            "checksum mismatch\n  expected {SUITE_SHA256}\n  got      {digest}\n\
             The archive is not the pinned release. A published figure's\n\
             denominator must not move underneath it."
        ));
    }
    println!("verified {digest}");

    let status = Command::new("tar")
        .arg("xzf")
        .arg(&tarball)
        .arg("-C")
        .arg(&root)
        .status()
        .map_err(|e| format!("could not run tar: {e}"))?;
    if !status.success() {
        return Err(format!("tar failed: {status}"));
    }
    println!("extracted to {}", root.display());
    Ok(())
}
