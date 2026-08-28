#!/usr/bin/env bash
#
# Publish this crate to crates.io from a laptop.
#
# CI publishes on a tag once CARGO_REGISTRY_TOKEN exists on the
# repository (.github/workflows/release.yml). Until then this script is
# the release path, and it runs the same checks the workflow does, in
# the same order, so switching over changes who runs them and nothing
# else.
#
# crates.io cannot be un-published, only yanked. Everything below
# happens before the upload, and any failure stops it.
set -euo pipefail
# Resolve the repository from this script's own location, not from
# the caller's. `git rev-parse` reads the *current* directory, so
# running this by absolute path from anywhere else failed with
# "fatal: not a git repository".
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

CRATE="xmlschema"
DEPS=(oxml)
PKG_ARG=()

version="$(cargo metadata --no-deps --format-version 1 \
  | jq -r --arg n "$CRATE" '.packages[] | select(.name==$n) | .version')"
echo "== $CRATE $version =="

# 1. Releasing anything other than main's tip publishes a tree nobody
#    reviewed.
branch="$(git rev-parse --abbrev-ref HEAD)"
[ "$branch" = "main" ] || { echo "on $branch, not main"; exit 1; }
git fetch origin --quiet
[ "$(git rev-parse HEAD)" = "$(git rev-parse origin/main)" ] \
  || { echo "main is not level with origin/main"; exit 1; }
[ -z "$(git status --porcelain)" ] \
  || { echo "working tree is dirty; cargo publish would package it"; exit 1; }

# 2. The suite ships one version across six crates, and a crate whose
#    dependency is not yet live cannot resolve. The CI workflow waits;
#    here, say so and stop.
for dep in "${DEPS[@]}"; do
  if curl -sf -H "User-Agent: xmlschema publish script (https://github.com/sebastienrousseau/xmlschema)" "https://crates.io/api/v1/crates/$dep/$version" >/dev/null; then
    echo "  dependency $dep $version is live"
  else
    echo "  dependency $dep $version is NOT on crates.io yet -- publish it first"
    exit 1
  fi
done

# 3. Everything CI runs.
./scripts/gate.sh

# 4. What the upload would contain.
cargo publish --dry-run "${PKG_ARG[@]}"

echo
read -r -p "Publish $CRATE $version to crates.io? This cannot be undone. [y/N] " reply
[ "$reply" = "y" ] || { echo "stopped"; exit 1; }

cargo publish "${PKG_ARG[@]}"

# 5. crates.io indexes asynchronously, so a publish that returned
#    success is not yet one the next crate can depend on.
echo "waiting for the index"
for i in $(seq 1 30); do
  if curl -sf -H "User-Agent: xmlschema publish script (https://github.com/sebastienrousseau/xmlschema)" "https://crates.io/api/v1/crates/$CRATE/$version" >/dev/null; then
    echo "$CRATE $version is live"
    exit 0
  fi
  sleep 10
done
echo "published, but not indexed after five minutes -- check crates.io"
