#!/usr/bin/env bash
# The upgrade check: what a newer gsd-core tag changes against the tracked release (D-r44-19).
#   usage: upgrade.sh <gsd-core-dir> <tag>
# Extracts the tag's inventory manifest, capability ids, config manifest and the list of changed files into a temp
# dir, then runs the `write_the_backlog` test of @gsd/core, which writes gsd/parity/backlog/<tag>.md. Read-only on
# the gsd-core checkout; needs git and cargo, not the network.
set -euo pipefail

if [ "$#" -ne 2 ]; then
  echo "usage: upgrade.sh <gsd-core-dir> <tag>" >&2
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
for ref in "$tracked" "$tag"; do
  git -C "$repo" rev-parse --verify --quiet "$ref^{commit}" > /dev/null || {
    echo "upgrade.sh: $repo has no tag $ref" >&2
    exit 2
  }
done

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
git -C "$repo" show "$tag:docs/INVENTORY-MANIFEST.json" > "$tmp/INVENTORY-MANIFEST.json"
git -C "$repo" show "$tag:gsd-core/bin/shared/config-schema.manifest.json" > "$tmp/config-schema.manifest.json"
git -C "$repo" ls-tree --name-only "$tag" capabilities/ |
  sed 's|^capabilities/||' |
  awk 'BEGIN { printf "[" } { printf "%s\"%s\"", (NR > 1 ? "," : ""), $0 } END { print "]" }' > "$tmp/capability-ids.json"
git -C "$repo" diff --name-status --no-renames "$tracked" "$tag" -- \
  commands agents hooks gsd-core src capabilities tests/fixtures/gate-cutover > "$tmp/changes.txt"

cd "$here/.."
GSD_UPGRADE_DIR=$tmp GSD_UPGRADE_TAG=$tag \
  cargo test --locked -p gsd-core --test upgrade -- --ignored write_the_backlog
echo "wrote gsd/parity/backlog/$tag.md"
