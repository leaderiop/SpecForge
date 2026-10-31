#!/bin/sh
# Records what gsd-tools answers over the vault fixture about its threats, into goldens/vault/. Run by hand (needs node
# and a built gsd-core), never in CI.
#
#   git -C ~/Projects/OpenSource/gsd-core archive v1.18.0 | tar -x -C $TMP/gsd-core-1.18.0
#   (cd $TMP/gsd-core-1.18.0 && npm ci && npm run build:lib)
#   GSD_CORE_DIR=$TMP/gsd-core-1.18.0 gsd/parity/record-security.sh
#
# As record.sh: the verbs run on a copy of the fixture that is its own git repository, timestamps, dates and hashes are
# normalised, the goldens are replaced only when every verb succeeded, and a verb that errors fails the script: fix the
# fixture, never a golden. The fixture's .planning/ is hand-written (plan 16, decision U3); the verbs vouch for it.
set -eu

: "${GSD_CORE_DIR:?set GSD_CORE_DIR to a gsd-core v1.18.0 checkout whose lib is built}"
here=$(cd "$(dirname "$0")" && pwd)
fixture="$here/fixtures/vault"
out="$here/goldens/vault"
tools="$GSD_CORE_DIR/gsd-core/bin/gsd-tools.cjs"

version=$(node -p "require('$GSD_CORE_DIR/package.json').version")
[ "$version" = "1.18.0" ] || { echo "record-security.sh: GSD_CORE_DIR is gsd-core $version, the tracked release is 1.18.0" >&2; exit 1; }
[ -f "$tools" ] || { echo "record-security.sh: $tools is missing" >&2; exit 1; }

work=$(mktemp -d)
staged="$work.staged"
rm -rf "$staged"
mkdir -p "$staged"
trap 'rm -rf "$work" "$staged"' EXIT
git -C "$fixture" ls-files -z --cached --others --exclude-standard | tar -C "$fixture" -c --null -T - -f - | tar -x -C "$work"
(
  cd "$work"
  git init -q -b main
  git -c user.name=vault -c user.email=vault@example.com add -A
  git -c user.name=vault -c user.email=vault@example.com commit -q -m "vault fixture"
)

# Reads JSON on stdin; prints it with ISO timestamps, dates, 40-hex hashes and the copy and checkout paths replaced, and a final newline.
real=$(cd "$work" && pwd -P)
core=$(cd "$GSD_CORE_DIR" && pwd -P)
normalise() {
  WORK="$work" REAL="$real" CORE="$core" node -e '
    let text = "";
    process.stdin.on("data", (c) => (text += c)).on("end", () => {
      const walk = (v) => {
        if (typeof v === "string") {
          return v
            .replace(/\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?Z/g, "<timestamp>")
            .replace(/\b[0-9a-f]{40}\b/g, "<sha>")
            .split(process.env.REAL).join("<project>")
            .split(process.env.WORK).join("<project>")
            .split(process.env.CORE).join("<gsd-core>");
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

# Sets workflow.security_enforcement in the copy's .planning/config.json.
enforcement() {
  node -e '
    const fs = require("fs");
    const file = process.argv[1] + "/.planning/config.json";
    const config = JSON.parse(fs.readFileSync(file, "utf8"));
    config.workflow.security_enforcement = process.argv[2] === "true";
    fs.writeFileSync(file, JSON.stringify(config, null, 2) + "\n");
  ' "$work" "$1"
}

predicate='{"kind":"artifact-frontmatter-equals","artifact":"SECURITY.md","field":"threats_open","equals":0}'

for point in plan:pre verify:post ship:pre; do
  record "render-hooks-$(echo "$point" | tr ':' '-')" loop render-hooks "$point" --raw
done
enforcement false
for point in plan:pre verify:post ship:pre; do
  record "render-hooks-$(echo "$point" | tr ':' '-')-off" loop render-hooks "$point" --raw
done
enforcement true

record init-execute-phase-1 init execute-phase 1
record init-plan-phase-1 init plan-phase 1
record init-execute-phase-2 init execute-phase 2
record check-predicate-1 check predicate --predicate "$predicate" --phase-dir .planning/phases/01-core
record check-predicate-2 check predicate --predicate "$predicate" --phase-dir .planning/phases/02-sharing

record config-get-security-enforcement query config-get workflow.security_enforcement
record config-get-security-asvs-level query config-get workflow.security_asvs_level
record config-get-security-block-on query config-get workflow.security_block_on
record config-get-injection-blocking query config-get security.injection_blocking

# The audit trail: the first call appends a block, the second finds the same rows and appends nothing.
for n in 1 2; do
  record "append-audit-$n" query verification.append-audit .planning/phases/01-core/01-SECURITY.md \
    --heading "Security Audit" --rows '{"Threats found":3,"Closed":2,"Open":1}' --date 2026-10-31
done

mkdir -p "$out"
rm -f "$out"/*.json
cp "$staged"/*.json "$out"/
echo "recorded $(ls "$staged" | wc -l | tr -d ' ') goldens in $out (gsd-core $version)"
