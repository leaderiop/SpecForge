#!/bin/sh
# Records what gsd-tools answers over the taskboard fixture into goldens/taskboard/.
#
#   GSD_CORE_DIR=<a v1.18.0 checkout with its lib built> gsd/parity/record.sh
#
# GSD_CORE_DIR is the root of a gsd-core checkout at the tracked tag (`npm ci && npm run build:lib` there). The verbs run
# on a copy of the fixture that is its own git repository, so what they say about git (stale worktrees, commit hashes)
# does not depend on the machine, and timestamps and hashes are normalised: a second run changes nothing.
set -eu

: "${GSD_CORE_DIR:?set GSD_CORE_DIR to a gsd-core v1.18.0 checkout whose lib is built}"
here=$(cd "$(dirname "$0")" && pwd)
fixture="$here/fixtures/taskboard"
out="$here/goldens/taskboard"
tools="$GSD_CORE_DIR/gsd-core/bin/gsd-tools.cjs"

version=$(node -p "require('$GSD_CORE_DIR/package.json').version")
[ "$version" = "1.18.0" ] || { echo "record.sh: GSD_CORE_DIR is gsd-core $version, the tracked release is 1.18.0" >&2; exit 1; }
[ -f "$tools" ] || { echo "record.sh: $tools is missing" >&2; exit 1; }

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
# The verbs write to a staging directory; the goldens are replaced only when every verb succeeded.
staged="$work.staged"
rm -rf "$staged"
mkdir -p "$staged"
trap 'rm -rf "$work" "$staged"' EXIT
# The fixture's files git tracks or would track (not a local junit.xml from `npm test`).
git -C "$fixture" ls-files -z --cached --others --exclude-standard | tar -C "$fixture" -c --null -T - -f - | tar -x -C "$work"
(
  cd "$work"
  git init -q -b main
  git -c user.name=taskboard -c user.email=taskboard@example.com add -A
  git -c user.name=taskboard -c user.email=taskboard@example.com commit -q -m "taskboard fixture"
)

# Reads JSON on stdin; prints it with ISO timestamps, dates and 40-hex hashes replaced and a final newline.
normalise() {
  node -e '
    let text = "";
    process.stdin.on("data", (c) => (text += c)).on("end", () => {
      const walk = (v) => {
        if (typeof v === "string") {
          return v
            .replace(/\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?Z/g, "<timestamp>")
            .replace(/\b[0-9a-f]{40}\b/g, "<sha>");
        }
        if (Array.isArray(v)) return v.map(walk);
        if (v && typeof v === "object") {
          return Object.fromEntries(Object.entries(v).map(([k, x]) => [k, k === "date" ? "<date>" : walk(x)]));
        }
        return v;
      };
      process.stdout.write(JSON.stringify(walk(JSON.parse(text)), null, 2) + "\n");
    });
  '
}

record() {
  name=$1
  shift
  # Two steps, not a pipe: POSIX sh has no pipefail, and a failing gsd-tools that still prints JSON must not be recorded.
  node "$tools" --cwd "$work" "$@" >"$staged/$name.raw"
  normalise <"$staged/$name.raw" >"$staged/$name.json"
  rm "$staged/$name.raw"
}

record roadmap-analyze roadmap analyze
record phase-plan-index-1 phase-plan-index 1
record phase-plan-index-2 phase-plan-index 2
record state-json state json
record progress-json progress json
record validate-health validate health
mkdir -p "$out"
cp "$staged"/*.json "$out"/
echo "recorded $(ls "$staged" | wc -l | tr -d ' ') goldens in $out (gsd-core $version)"
