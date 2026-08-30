#!/usr/bin/env python3
"""Fail if the crate would ship a document making a claim about itself
that is not true of itself.

`cargo publish` uploads far more than `src/`. With no `include` list in
Cargo.toml it packages every tracked file, so the assurance case, the
README and the roadmap all reach crates.io -- where they are immutable.

oxml-json 0.0.8 shipped an assurance case inherited from oxml-lsp
claiming "21 tests over `analyse()`" and a quadratic pass fixed by
benchmark. This crate has no `analyse()` and never had that benchmark.
The correction landed twelve minutes after the upload, which is eleven
minutes and fifty seconds too late: crates.io versions cannot be
edited, only superseded.

The check: every function this crate's own documents claim to have
must actually exist in `src/`. A document may still name another
crate's API, but only qualified -- `oxml::parse` is a reference,
`analyse()` is a claim.
"""

import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent

# Only the assurance case. It is a claims document by definition --
# every line is evidence offered for this crate -- so a bare `name()`
# in it is a claim that this crate has it.
#
# READMEs and roadmaps are prose and were tried first. They produce
# false positives that a gate must not have: oxml-cli's README says
# "This is XPath 1.0, so no `matches()`", which is a *negation*, and
# oxml-wasm's documents `free()` and `rootName()`, which are the
# JavaScript names wasm-bindgen generates rather than Rust functions.
# A check that cries wolf on correct documentation gets switched off.
DESCRIBES_THIS_CRATE = ("doc/ASSURANCE-CASE.md",)

# Bare names that are not this crate's API and never will be.
IGNORE = {
    # Rust and tooling vocabulary that reads like a call.
    "main", "fn", "unsafe", "await", "match", "if", "else", "loop",
    "cargo", "rustc", "clippy", "rustfmt", "test", "tests", "bench",
    "assert", "assert_eq", "debug_assert", "println", "write", "format",
    "panic", "unwrap", "expect", "clone", "into", "from", "to_string",
    "len", "is_empty", "iter", "collect", "map", "filter", "count",
}


def cargo() -> str:
    """Where cargo is, saying so plainly when it is nowhere.

    A bare `cargo` is right on CI and in a normal shell. It is not
    right in every shell -- a rustup install puts the real binary in
    ~/.cargo/bin, and a PATH that lost that entry produces a
    FileNotFoundError traceback rather than a sentence.
    """
    import os
    import shutil

    found = shutil.which(os.environ.get("CARGO", "cargo"))
    if found:
        return found
    fallback = pathlib.Path.home() / ".cargo" / "bin" / "cargo"
    if fallback.is_file():
        return str(fallback)
    sys.exit("cargo is not on PATH and ~/.cargo/bin/cargo does not exist")


def run(*args: str) -> str:
    args = (cargo(),) + args[1:] if args and args[0] == "cargo" else args
    proc = subprocess.run(args, cwd=ROOT, capture_output=True, text=True, check=False)
    if proc.returncode != 0:
        sys.exit(f"command failed: {' '.join(args)}\n{proc.stderr[-2000:]}")
    return proc.stdout


def packaged() -> list[str]:
    # --allow-dirty on purpose: this is a *pre-publish* check, so the
    # question is what the tree in front of you would ship, including
    # anything not yet committed. Refusing on a dirty tree would mean
    # the check could not run at the moment it is most useful.
    out = run("cargo", "package", "--list", "--allow-dirty")
    return [ln.strip() for ln in out.splitlines() if ln.strip()]


def defined_functions() -> set[str]:
    names: set[str] = set()
    # A single-crate repo keeps sources in src/; oxml is a workspace and
    # keeps them in crates/<name>/src. Take whichever exists rather
    # than assuming, because assuming produced "found no functions in
    # src/" on the one repo with the most functions.
    roots = [ROOT / "src"] + sorted((ROOT / "crates").glob("*/src"))
    for src in (r for r in roots if r.is_dir()):
        for path in src.rglob("*.rs"):
            text = path.read_text()
            names |= set(re.findall(r"\bfn\s+([A-Za-z_][A-Za-z0-9_]*)", text))
            # Types count too: a method is reached through one.
            names |= set(re.findall(r"\bstruct\s+([A-Za-z_][A-Za-z0-9_]*)", text))
            names |= set(re.findall(r"\benum\s+([A-Za-z_][A-Za-z0-9_]*)", text))
    return names


def main() -> int:
    files = packaged()
    print(f"the package would contain {len(files)} files")

    # A document that exists but is not packaged is not the failure this
    # check is about -- but it means the check would not have protected
    # it, so say so.
    for name in DESCRIBES_THIS_CRATE:
        if (ROOT / name).exists() and name not in files:
            print(f"  note: {name} exists but is not packaged")

    have = defined_functions()
    if not have:
        sys.exit("found no functions in src/ or crates/*/src -- the check is not working")

    problems: list[str] = []
    checked = 0
    for name in DESCRIBES_THIS_CRATE:
        path = ROOT / name
        if not path.exists() or name not in files:
            continue
        for number, line in enumerate(path.read_text().splitlines(), 1):
            # `name()` in backticks: a claim that this crate has it.
            # `other::name` is qualified, so it is a reference, not a
            # claim, and is deliberately not matched.
            for call in re.findall(r"`([A-Za-z_][A-Za-z0-9_]*)\(\)`", line):
                if call in IGNORE:
                    continue
                checked += 1
                if call not in have:
                    problems.append(f"{name}:{number} claims `{call}()`, which src/ does not define")

    print(f"{checked} documented API claims checked against src/")
    if problems:
        print("\nclaims this crate cannot support:")
        for p in problems:
            print(f"  {p}")
        print(
            "\nThese ship to crates.io and cannot be edited afterwards.\n"
            "Either the document belongs to another crate, or the API is gone."
        )
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
