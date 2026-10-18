#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# Validate this crate's release package from a laptop, without uploading.
#
# Publication belongs to the reviewed CI release workflow. This helper
# checks the reviewed main tree and package; it has no upload mode.
#
# crates.io versions are immutable; failures must stop before publication.
set -euo pipefail
# Resolve the repository from this script's own location, not from
# the caller's. `git rev-parse` reads the *current* directory, so
# running this by absolute path from anywhere else failed with
# "fatal: not a git repository".
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

CRATE="xmlschema"

version="$(cargo metadata --no-deps --format-version 1 --locked \
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

# 2. Run the local gate. Cargo's dry run below also requires every
#    registry dependency to be resolvable at the declared family version.
./scripts/gate.sh

# 3. What the upload would contain, using the reviewed dependency lock.
cargo publish --dry-run --locked

echo "Dry run complete for $CRATE $version. Nothing was published."
echo "Publication requires the reviewed release workflow."
