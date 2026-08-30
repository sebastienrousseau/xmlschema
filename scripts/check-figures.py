#!/usr/bin/env python3
"""Fail if a documented test count is not the number the suite produces.

Every crate here states how many tests it has, in the README and in
doc/TESTING.md and doc/ASSURANCE-CASE.md. Nothing measured those
numbers, so they decayed: oxml stated three different totals in three
places (403+26, 367+22, 385+22) when the real figure was 428+26;
oxml-mcp claimed 57 tests while having 40, because the JSON tests left
with the code when it moved to its own crate.

A count that is merely stale is a small thing. A count that is *higher*
than reality is a claim of coverage that does not exist, and it is the
one an assurance case rests on.

Conformance-suite sizes (2,585 W3C tests, 39,420 XSD tests) are not
cargo tests and are not checked here; they are written with thousands
separators, which is the discriminator this uses.
"""

import pathlib
import re
import shutil
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
# The scope the crate's own TESTING.md documents. oxml is a workspace
# and counts it; the single-crate repos document plain `cargo test`,
# and counting their conformance harness too would make the published
# figure describe something the reader cannot reproduce.
TEST_ARGS = ["test", "--all-features"]


def cargo() -> str:
    found = shutil.which("cargo")
    if found:
        return found
    fallback = pathlib.Path.home() / ".cargo" / "bin" / "cargo"
    if fallback.is_file():
        return str(fallback)
    sys.exit("cargo is not on PATH and ~/.cargo/bin/cargo does not exist")


def measure() -> tuple[int, int]:
    """(tests, doctests) as cargo actually reports them."""
    proc = subprocess.run(
        [cargo()] + TEST_ARGS, cwd=ROOT, capture_output=True, text=True, check=False
    )
    if proc.returncode != 0:
        sys.exit(
            "the test suite does not pass, so its size cannot be checked:\n"
            + proc.stdout[-1500:]
        )
    totals = [int(n) for n in re.findall(r"^test result: ok\. (\d+) passed", proc.stdout, re.M)]

    doc = subprocess.run(
        [cargo(), "test", "--doc", "--workspace", "--all-features"],
        cwd=ROOT, capture_output=True, text=True, check=False,
    )
    doctests = sum(int(n) for n in re.findall(r"^test result: ok\. (\d+) passed", doc.stdout, re.M))
    return sum(totals) - doctests, doctests


def documents() -> list[pathlib.Path]:
    out = [ROOT / "README.md"]
    out += sorted((ROOT / "doc").glob("*.md")) if (ROOT / "doc").is_dir() else []
    return [p for p in out if p.exists()]


def main() -> int:
    tests, doctests = measure()
    print(f"measured: {tests} tests, {doctests} doctests")

    # Only phrasings that mean *this crate's suite*.
    #
    # A bare "N tests" is not enough. These documents also say "28
    # tests reach no decision" about W3C conformance, "209 tests, zero
    # failures" about one historical run, and "a loader silently
    # dropped 159 tests". None of those is the size of this suite, and
    # a check that flagged them would be teaching people to ignore it.
    #
    # What a real claim looks like: a count paired with a doctest
    # count, or "N tests over/covering/in ..." introducing what they
    # cover.
    pair = re.compile(r"(?<![\d,])(\d{1,4})\s+tests?\b.{0,20}?(?<![\d,])(\d{1,3}|a)\s+doctests?\b")
    intro = re.compile(r"(?<![\d,])(\d{1,4})\s+tests?(?::|[,]?\s+(?:over|covering|in)\b)")

    wrong: list[str] = []
    checked = 0
    for path in documents():
        rel = path.relative_to(ROOT)
        for number, line in enumerate(path.read_text().splitlines(), 1):
            if "figures-check: ignore" in line:
                continue
            for t, d in pair.findall(line):
                checked += 2
                if int(t) != tests:
                    wrong.append(f"{rel}:{number} says {t} tests, measured {tests}")
                # "and a doctest" is a legitimate way to say one.
                want_d = 1 if d == "a" else int(d)
                if want_d != doctests:
                    wrong.append(f"{rel}:{number} says {d} doctests, measured {doctests}")
            if not pair.search(line):
                for t in intro.findall(line):
                    checked += 1
                    if int(t) != tests:
                        wrong.append(f"{rel}:{number} says {t} tests, measured {tests}")

    if checked == 0:
        print("no documented test counts to check")
        return 0
    print(f"{checked} documented figures checked")
    if wrong:
        print("\nfigures that do not match the suite:")
        for w in wrong:
            print(f"  {w}")
        print(
            "\nUpdate the document, or add `figures-check: ignore` to the line\n"
            "if the number genuinely means something else."
        )
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
