// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 xmlschema. All rights reserved.

//! SHA-256, implemented here to keep this crate dependency-free.
//!
//! This verifies a 15 MB archive against a hash pinned in the
//! downloader. A digest function that is subtly wrong does not fail
//! loudly -- it quietly accepts whatever it is given, which is the
//! one thing the pin exists to prevent. It lives in the library
//! rather than the binary so that it can be tested.

// FIPS 180-4 names these variables `h`, `w`, `a`..`h` and `k`. Renaming
// them to satisfy a lint would make the code harder to check against
// the specification, which is the only way anyone verifies a hash
// implementation. The function is long for the same reason: the
// compression loop is one block in the standard and splitting it would
// obscure rather than clarify.
#[allow(
    clippy::many_single_char_names,
    clippy::too_many_lines,
    clippy::format_collect
)]
#[must_use]
pub fn sha256(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a_2f98,
        0x7137_4491,
        0xb5c0_fbcf,
        0xe9b5_dba5,
        0x3956_c25b,
        0x59f1_11f1,
        0x923f_82a4,
        0xab1c_5ed5,
        0xd807_aa98,
        0x1283_5b01,
        0x2431_85be,
        0x550c_7dc3,
        0x72be_5d74,
        0x80de_b1fe,
        0x9bdc_06a7,
        0xc19b_f174,
        0xe49b_69c1,
        0xefbe_4786,
        0x0fc1_9dc6,
        0x240c_a1cc,
        0x2de9_2c6f,
        0x4a74_84aa,
        0x5cb0_a9dc,
        0x76f9_88da,
        0x983e_5152,
        0xa831_c66d,
        0xb003_27c8,
        0xbf59_7fc7,
        0xc6e0_0bf3,
        0xd5a7_9147,
        0x06ca_6351,
        0x1429_2967,
        0x27b7_0a85,
        0x2e1b_2138,
        0x4d2c_6dfc,
        0x5338_0d13,
        0x650a_7354,
        0x766a_0abb,
        0x81c2_c92e,
        0x9272_2c85,
        0xa2bf_e8a1,
        0xa81a_664b,
        0xc24b_8b70,
        0xc76c_51a3,
        0xd192_e819,
        0xd699_0624,
        0xf40e_3585,
        0x106a_a070,
        0x19a4_c116,
        0x1e37_6c08,
        0x2748_774c,
        0x34b0_bcb5,
        0x391c_0cb3,
        0x4ed8_aa4a,
        0x5b9c_ca4f,
        0x682e_6ff3,
        0x748f_82ee,
        0x78a5_636f,
        0x84c8_7814,
        0x8cc7_0208,
        0x90be_fffa,
        0xa450_6ceb,
        0xbef9_a3f7,
        0xc671_78f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09_e667,
        0xbb67_ae85,
        0x3c6e_f372,
        0xa54f_f53a,
        0x510e_527f,
        0x9b05_688c,
        0x1f83_d9ab,
        0x5be0_cd19,
    ];
    let mut msg = data.to_vec();
    let bit_len = (data.len() as u64) * 8;
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    // `chunks_exact` rather than `as_chunks`: clippy 1.98 prefers the
    // latter, but `slice_as_chunks` is unstable until well after this
    // crate's MSRV of 1.86, so taking the lint's advice breaks the MSRV
    // build. The lint is suppressed rather than obeyed.
    #[allow(clippy::chunks_exact_to_as_chunks)]
    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (i, word) in w.iter_mut().enumerate().take(16) {
            let b: [u8; 4] = [
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ];
            *word = u32::from_be_bytes(b);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7)
                ^ w[i - 15].rotate_right(18)
                ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17)
                ^ w[i - 2].rotate_right(19)
                ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 =
                e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 =
                a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (dst, src) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *dst = dst.wrapping_add(src);
        }
    }
    h.iter().map(|w| format!("{w:08x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `n` repetitions of `a`, the input the published vectors use.
    fn repeat_a(n: usize) -> Vec<u8> {
        vec![b'a'; n]
    }

    /// The published FIPS 180-2 / RFC 6234 vectors.
    ///
    /// A hand-rolled digest that is never checked against a known
    /// answer is decoration: it produces 64 plausible hex characters
    /// for any input, and the pin it backs is worthless.
    #[test]
    fn matches_the_published_vectors() {
        for (input, want) in [
            (
                "",
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            ),
            (
                "abc",
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            ),
            (
                "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",
                "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
            ),
            (
                "abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmn\
                 hijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu",
                "cf5b16a778af8380036ce59e7b0492370b249b11e8f07a51afac45037afee9d1",
            ),
        ] {
            assert_eq!(sha256(input.as_bytes()), want, "input {input:?}");
        }
    }

    #[test]
    fn the_message_length_is_appended_as_bits_not_bytes() {
        // One million `a`s: the vector that catches a 32-bit length
        // field, a byte/bit confusion, or a length that overflows.
        let million = repeat_a(1_000_000);
        assert_eq!(
            sha256(&million),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
    }

    #[test]
    fn padding_is_right_at_every_block_boundary() {
        // 55 bytes is the largest message whose padding fits in one
        // block; 56 forces a second one. Getting `while len % 64 != 56`
        // wrong shows up here and almost nowhere else, because every
        // short test vector happens to sit safely inside one block.
        //
        // Expected digests are the published values for `a`-repeated
        // messages at each boundary.
        for (n, want) in [
            (
                55,
                "9f4390f8d30c2dd92ec9f095b65e2b9ae9b0a925a5258e241c9f1e910f734318",
            ),
            (
                56,
                "b35439a4ac6f0948b6d6f9e3c6af0f5f590ce20f1bde7090ef7970686ec6738a",
            ),
            (
                63,
                "7d3e74a05d7db15bce4ad9ec0658ea98e3f06eeecf16b4c6fff2da457ddc2f34",
            ),
            (
                64,
                "ffe054fe7ae0cb6dc65c3af9b61d5209f439851db43d0ba5997337df154668eb",
            ),
            (
                65,
                "635361c48bb9eab14198e76ea8ab7f1a41685d6ad62aa9146d301d4f17eb0ae0",
            ),
        ] {
            assert_eq!(sha256(&repeat_a(n)), want, "{n} bytes");
        }
    }

    #[test]
    fn the_digest_is_lowercase_hex_of_exactly_32_bytes() {
        // The pin is compared as a string, so a leading zero dropped by
        // a `{:x}` without a width would silently never match -- or,
        // worse, match the wrong thing.
        for n in [0usize, 1, 31, 32, 100] {
            let d = sha256(&repeat_a(n));
            assert_eq!(d.len(), 64, "{n} bytes produced {d:?}");
            assert!(
                d.chars()
                    .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
                "{d:?} is not lowercase hex"
            );
        }
        // A digest whose first byte is 0x6e and whose input is a
        // single NUL: catches a `{:x}` that drops a leading zero from
        // a hex pair, which would shorten the string and never match
        // the pin.
        assert_eq!(
            sha256(&[0u8; 1]),
            "6e340b9cffb37a989ca544e6bb780a2c78901d3fb33738768511a30617afa01d"
        );
    }

    #[test]
    fn a_single_flipped_bit_changes_the_digest() {
        // The property the pin depends on. Without it, a tampered
        // archive of the same length could pass.
        let a = repeat_a(1024);
        let mut b = a.clone();
        b[512] ^= 0x01;
        assert_ne!(sha256(&a), sha256(&b));
    }
}
