#!/usr/bin/env bash
# The upgrade check: what a newer gsd-core tag changes against the tracked release (D-r44-19).
#   usage: upgrade.sh <gsd-core-dir> <tag> [<from-tag>]
# Extracts the inventory manifest, capability ids and config manifest of <tag> and of <from-tag> (default: the tracked
# release), and the list of files changed between them, into a temp dir, then runs the `write_the_backlog` test of
# @gsd/core, which writes gsd/parity/backlog/<tag>.md. <from-tag> re-writes a backlog after `tracks` has moved.
# Read-only on the gsd-core checkout (git show/diff only, nothing of it runs); needs git and cargo, not the network.
set -euo pipefail

if [ "$#" -lt 2 ] || [ "$#" -gt 3 ]; then
  echo "usage: upgrade.sh <gsd-core-dir> <tag> [<from-tag>]" >&2
  exit 2
fi
repo=$1
tag=$2
here=$(cd "$(dirname "$0")" && pwd)

tracked=$(sed -n 's/^tracks = "gsd-core@\(.*\)"$/v\1/p' "$here/../core/Cargo.toml")
if [ -z "$tracked" ]; then
  echo "upgrade.sh: gsd/core/Cargo.toml has no tracks" >&2
  exit 2
fi
from=${3:-$tracked}
for ref in "$from" "$tag"; do
  git -C "$repo" rev-parse --verify --quiet "$ref^{commit}" > /dev/null || {
    echo "upgrade.sh: $repo has no tag $ref" >&2
    exit 2
  }
done

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
extract() { # <ref> <dir>
  mkdir -p "$2"
  git -C "$repo" show "$1:docs/INVENTORY-MANIFEST.json" > "$2/INVENTORY-MANIFEST.json"
  git -C "$repo" show "$1:gsd-core/bin/shared/config-schema.manifest.json" > "$2/config-schema.manifest.json"
  git -C "$repo" ls-tree --name-only "$1" capabilities/ |
    sed 's|^capabilities/||' |
    awk 'BEGIN { printf "[" } { printf "%s\"%s\"", (NR > 1 ? "," : ""), $0 } END { print "]" }' > "$2/capability-ids.json"
}
extract "$from" "$tmp/from"
extract "$tag" "$tmp/to"
# The upstream suites the snapshot copies (its tests/*.test.cjs), whose cases the extensions port.
suites=()
for suite in "$here/upstream/$tracked"/tests/*.test.cjs; do
  if [ -e "$suite" ]; then suites+=("tests/${suite##*/}"); fi
done
git -C "$repo" diff --name-status --no-renames "$from" "$tag" -- \
  commands agents hooks gsd-core src capabilities tests/fixtures/gate-cutover ${suites[@]+"${suites[@]}"} > "$tmp/changes.txt"

cd "$here/.."
GSD_UPGRADE_DIR=$tmp GSD_UPGRADE_TAG=$tag GSD_UPGRADE_FROM=$from \
  cargo test --locked -p gsd-core --test upgrade -- --ignored write_the_backlog
echo "wrote gsd/parity/backlog/$tag.md"
