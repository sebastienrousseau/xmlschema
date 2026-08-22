# Contributing to xmlschema

Thank you for considering a contribution.

## Ground rules

- **No `unsafe`.** The crate is `#![forbid(unsafe_code)]` and that is
  not negotiable — it is a large part of what the library is for.
- **Every change is tested.** A bug fix comes with a test that fails
  before it and passes after. "It compiles" is necessary, not
  sufficient.
- **Benchmarks back performance claims.** If a change is described as
  faster, the pull request should say by how much, measured.

## Getting set up

```bash
git clone https://github.com/sebastienrousseau/xmlschema
cd xmlschema
cargo test
```

## Before opening a pull request

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo doc --no-deps
```

All four must pass. CI runs the same commands, so a green local run
means a green pipeline.

## Commit messages

[Conventional Commits](https://www.conventionalcommits.org/). The body
matters more than the subject: explain what was wrong and why the fix
is right, not just what changed.

## Reporting a security issue

Please do not open a public issue. See [SECURITY.md](SECURITY.md).
