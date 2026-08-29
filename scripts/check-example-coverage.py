#!/usr/bin/env python3
"""Fail if any public function is not executed by at least one example.

Ported from `oxml`, which is where this check was written and where
the reasoning below was learned.

The README claims the examples cover the public API. That claim decays
silently: a function added in one commit and documented in none is
indistinguishable, from the outside, from one the examples exercise.
This measures it instead, by running the examples under coverage
instrumentation and checking the execution count at the line each
`pub fn` is declared on.

It deliberately checks *execution*, not mention. An example that names
a function in a comment does not count.
"""

import json
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
SRC = ROOT / "src"
MANIFEST = ROOT / "Cargo.toml"


def examples() -> list[str]:
    text = MANIFEST.read_text()
    return re.findall(r'^\[\[example\]\]\nname = "([^"]+)"', text, re.M)


def public_modules() -> set[str]:
    """Module files reachable from the crate's public API.

    A `pub fn` inside a private module is not public, and demanding an
    example for it would be demanding an example for something a caller
    cannot call. `src/json.rs` is exactly that case here.

    Only `lib.rs` and the modules it declares `pub mod` are checked.
    This is deliberately shallow -- a `pub mod` nested inside a private
    one would be missed -- because the crate has no such nesting and a
    check that models the whole visibility system would be harder to
    trust than the thing it checks.
    """
    lib = (SRC / "lib.rs").read_text()
    names = set(re.findall(r"^\s*pub mod (\w+);", lib, re.M))
    return {"lib.rs"} | {f"{n}.rs" for n in names}


def run(*args: str) -> str:
    proc = subprocess.run(
        args, cwd=ROOT, capture_output=True, text=True, check=False
    )
    if proc.returncode != 0:
        # Every command here must succeed. When a failing command was
        # ignored, an example that did not compile produced no coverage
        # data, the file it exercised was skipped for having none, and
        # the check reported success over an API nothing had run.
        sys.exit(
            f"command failed: {' '.join(args)}\n"
            f"{proc.stdout[-2000:]}\n{proc.stderr[-2000:]}"
        )
    return proc.stdout


def main() -> int:
    names = examples()
    if not names:
        sys.exit("no examples declared in Cargo.toml")
    print(f"running {len(names)} examples under instrumentation")

    run("cargo", "llvm-cov", "clean", "--workspace")
    for name in names:
        run(
            "cargo", "llvm-cov", "--no-report", "run",
            "-q", "-p", "xmlschema", "--example", name,
        )
    report = run("cargo", "llvm-cov", "report", "--json")

    # file -> {line number: execution count}
    #
    # Read from `--json` rather than `--text`. The text renderer omits
    # the `/path/file.rs:` header when the report covers exactly one
    # file, so a parser keyed on that header silently attributed no
    # coverage to anything -- and this check reports "no coverage data
    # at all" as a failure, so a single-file crate could never pass it.
    # Found porting the check to `oxml-cli`, whose public API is one
    # function in one file. The JSON export always names the file.
    counts: dict[str, dict[int, int]] = {}
    for entry in json.loads(report)["data"]:
        for item in entry["files"]:
            per_line: dict[int, int] = {}
            # A segment is [line, column, count, has_count, is_entry, is_gap].
            # Several may start on one line; the line is executed if any
            # of them ran, so keep the largest count.
            for line, _column, count, has_count, *_rest in item["segments"]:
                if has_count:
                    per_line[line] = max(per_line.get(line, 0), count)
            counts[item["filename"]] = per_line

    total = 0
    unexercised: list[str] = []
    reachable = public_modules()
    for path in sorted(SRC.rglob("*.rs")):
        if path.name not in reachable:
            continue
        rel = path.relative_to(ROOT)
        lines = path.read_text().splitlines()
        declared = [
            (n, t.strip())
            for n, t in enumerate(lines, 1)
            if t.strip().startswith(("pub fn ", "pub const fn "))
        ]
        keys = [k for k in counts if k.endswith(str(rel))]
        if not keys:
            # Not "nothing to check" -- a file with public functions and
            # no coverage data at all means the instrumented build never
            # reached it, which is a failure to measure, not a pass.
            if declared:
                unexercised.append(
                    f"{rel} has {len(declared)} public functions and no "
                    f"coverage data at all"
                )
                total += len(declared)
            continue
        for number, stripped in declared:
            total += 1
            count = counts[keys[0]].get(number)
            if count is None:
                # No region begins on the declaration line. For a
                # *generic* function that is exactly what an
                # uninstantiated one looks like: no caller, so no code
                # generated, so nothing to count.
                #
                # Reading "no region" as covered is how `lsp::serve`
                # passed this check while no example called it -- the
                # failure mode the check exists to prevent, occurring
                # inside the check.
                unexercised.append(
                    f"{rel}:{number} (never instantiated) "
                    f"{stripped.rstrip(' {')}"
                )
            elif count == 0:
                unexercised.append(f"{rel}:{number} {stripped.rstrip(' {')}")

    if total == 0:
        sys.exit("found no `pub fn` declarations -- the check is not working")

    print(f"{total - len(unexercised)}/{total} public functions exercised")
    if unexercised:
        print("\nnot reached by any example:")
        for item in unexercised:
            print(f"  {item}")
        print("\nAdd an example that calls it, or make it private.")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
