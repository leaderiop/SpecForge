#!/bin/sh
# Records what gsd-tools answers over the taskboard fixture, and what its own tests report, into goldens/taskboard/.
#
#   GSD_CORE_DIR=<a v1.18.0 checkout with its lib built> gsd/parity/record.sh
#
# GSD_CORE_DIR is the root of a gsd-core checkout at the tracked tag (`npm ci && npm run build:lib` there). The verbs run
# on a copy of the fixture that is its own git repository, so what they say about git (stale worktrees, commit hashes)
# does not depend on the machine, and timestamps and hashes are normalised: a second run changes nothing. The JUnit
# golden is `node --test`'s reporter output (paths, times and hostname are normalised); another Node major may word it
# differently, and then the diff is the Node release, not the port.
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
record verify-context-drift-2 query check.verify-context-drift 2
record decision-coverage-plan-2 query check.decision-coverage-plan .planning/phases/02-persistence .planning/phases/02-persistence/02-CONTEXT.md
record verify-plan-structure-01-01 query verify.plan-structure .planning/phases/01-core/01-01-PLAN.md
record verify-plan-structure-01-02 query verify.plan-structure .planning/phases/01-core/01-02-PLAN.md
record verify-plan-structure-02-01 query verify.plan-structure .planning/phases/02-persistence/02-01-PLAN.md
record verify-command-paths-1 query check.verify-command-paths 1
record verify-command-paths-2 query check.verify-command-paths 2
record verify-failure-directions-1 query check.verify-failure-directions 1
record verify-failure-directions-2 query check.verify-failure-directions 2
record gap-analysis-plan-post-1 query check.gap-analysis-plan-post .planning/phases/01-core TASK-01,TASK-02
record gap-analysis-plan-post-2 query check.gap-analysis-plan-post .planning/phases/02-persistence STORE-01,STORE-02
record audit-uat audit-uat
# The recorded tests: what `npm test` writes (node's JUnit reporter), with the machine's paths and timings out, so a
# second run changes nothing. `gsd-junit` links its tests to the twin's deliverables by name and project-relative file.
(cd "$work" && npm test >/dev/null)
real=$(cd "$work" && pwd -P)
sed -e "s|file=\"$real/|file=\"|g" -e 's/time="[^"]*"/time="0"/g' -e '/<!-- duration_ms /d' -e 's/ hostname="[^"]*"//g' "$work/junit.xml" >"$staged/junit.xml"
mkdir -p "$out"
cp "$staged"/*.json "$staged/junit.xml" "$out"/
echo "recorded $(ls "$staged" | wc -l | tr -d ' ') goldens in $out (gsd-core $version)"
