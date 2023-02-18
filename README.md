# xmlschema

XML Schema validator and data conversion library for Rust.

[![Made With Love][made-with-rust]][6]
[![Crates.io][crates-badge]][8]
[![Lib.rs][libs-badge]][10]
[![Docs.rs][docs-badge]][9]
[![License][license-badge]][2]

![divider][divider]

## Welcome to XML Schema 👋

![XML Schema Banner][xmlschema]

<!-- markdownlint-disable MD033 -->
<center>

**[Website][0]
• [Documentation][9]
• [Report Bug][3]
• [Request Feature][3]
• [Contributing Guidelines][4]**

</center>

<!-- markdownlint-enable MD033 -->

## Overview 📖

The xmlschema library is an implementation of [XML Schema](https://www.w3.org/2001/XMLSchema) for Rust. It provides a set of functions to validate XML documents against an XML Schema Definition (XSD) file and to convert XML documents to JSON and vice versa.

## Features ✨

This library aims to include the following features:

- Full XSD 1.0 and XSD 1.1 support
- Building of XML schema objects from XSD files
- Validation of XML instances against XSD schemas
- Decoding of XML data into Python data and to JSON
- Encoding of Rust data and JSON to XML

## Installation 📦

It takes just a few minutes to get up and running with `xmlschema`.

### Requirements

`xmlschema` requires Rust **1.67.1** or later.

### Documentation

> ℹ️ **Info:** Please check out our [website][0] for more information
and find our documentation on [docs.rs][9], [lib.rs][10] and
[crates.io][8].

## Usage 📖

To use `xmlschema` in your project, add the following to your
`Cargo.toml` file:

```toml
[dependencies]
xmlschema = "0.0.1"
```

Add the following to your `main.rs` file:

```rust
extern crate xmlschema;
use xmlschema::*;
```

then you can use the functions in your application code.

### Examples

`XML Schema` comes with a set of examples that you can use to get started. The
examples are located in the `examples` directory of the project. To run
the examples, clone the repository and run the following command in your
terminal from the project root directory.

```shell
cargo run --example xmlschema
```

## Semantic Versioning Policy 🚥

For transparency into our release cycle and in striving to maintain
backward compatibility, `XML Schema` follows [semantic versioning][7].

## License 📝

The project is licensed under the terms of both the MIT license and the
Apache License (Version 2.0).

- [Apache License, Version 2.0][1]
- [MIT license][2]

## Contribution 🤝

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms
or conditions.

![divider][divider]

## Acknowledgements 💙

A big thank you to all the awesome contributors of [Mini Functions][6]
for their help and support. A special thank you goes to the
[Rust Reddit](https://www.reddit.com/r/rust/) community for providing a
lot of useful suggestions on how to improve this project.

[0]: https://minifunctions.com/xmlschema
[1]: http://www.apache.org/licenses/LICENSE-2.0
[2]: http://opensource.org/licenses/MIT
[3]: https://github.com/sebastienrousseau/xmlschema/issues
[4]: https://raw.githubusercontent.com/sebastienrousseau/xmlschema/main/.github/CONTRIBUTING.md
[6]: https://github.com/sebastienrousseau/xmlschema/graphs/contributors
[7]: http://semver.org/
[8]: https://crates.io/crates/xmlschema
[9]: https://docs.rs/xmlschema
[10]: https://lib.rs/crates/xmlschema

[xmlschema]: https://raw.githubusercontent.com/sebastienrousseau/vault/main/assets/xmlschema/banners/banner-xmlschema-1597x377.svg "XML Schema Banner"
[crates-badge]: https://img.shields.io/crates/v/xmlschema.svg?style=for-the-badge 'Crates.io'
[divider]: https://raw.githubusercontent.com/sebastienrousseau/vault/main/assets/elements/divider.svg "divider"
[docs-badge]: https://img.shields.io/docsrs/xmlschema.svg?style=for-the-badge 'Docs.rs'
[libs-badge]: https://img.shields.io/badge/lib.rs-v0.0.2-orange.svg?style=for-the-badge 'Lib.rs'
[license-badge]: https://img.shields.io/crates/l/xmlschema.svg?style=for-the-badge 'License'
[made-with-rust]: https://img.shields.io/badge/rust-f04041?style=for-the-badge&labelColor=c0282d&logo=rust 'Made With Rust'
