#!/bin/sh
# Records what the ledger fixture's tests report and what gsd-tools answers over it into goldens/ledger/. Run by
# hand (needs cargo-nextest, and node 24 or later with a built gsd-core for the gsd-tools goldens), never in CI.
#
#   gsd/parity/record-ledger.sh                     # the two test reports only
#   git -C ~/Projects/OpenSource/gsd-core archive v1.19.0 | tar -x -C $TMP/gsd-core-1.19.0
#   (cd $TMP/gsd-core-1.19.0 && npm ci && npm run build:lib)
#   GSD_CORE_DIR=$TMP/gsd-core-1.19.0 gsd/parity/record-ledger.sh
#
# The ledger's CI is red by design: quick task 261105-tcl folded the total in an i32, so `total_beyond_i32_is_exact`
# fails. `rust-test.xml` is that run; `rust-test-before-quick.xml` is the run with the fold restored to an i64, the
# state phase 01's verifier saw, over which the twin's verdicts are recorded. As record.sh: the verbs run on a copy that
# is its own git repository, machine values are normalised, the goldens are replaced only when every step succeeded,
# and a step that errors fails the script: fix the fixture, never a golden.
set -eu

here=$(cd "$(dirname "$0")" && pwd)
fixture="$here/fixtures/ledger"
out="$here/goldens/ledger"

if [ -n "${GSD_CORE_DIR:-}" ]; then
  tools="$GSD_CORE_DIR/gsd-core/bin/gsd-tools.cjs"
  version=$(node -p "require('$GSD_CORE_DIR/package.json').version")
  [ "$version" = "1.19.0" ] || { echo "record-ledger.sh: GSD_CORE_DIR is gsd-core $version, the tracked release is 1.19.0" >&2; exit 1; }
  [ -f "$tools" ] || { echo "record-ledger.sh: $tools is missing" >&2; exit 1; }
fi

work=$(mktemp -d)
staged="$work.staged"
rm -rf "$staged"
mkdir -p "$staged"
trap 'rm -rf "$work" "$staged" "$work.target" "$work.nextest.toml" "$work.junit.xml"' EXIT
git -C "$fixture" ls-files -z --cached --others --exclude-standard | tar -C "$fixture" -c --null -T - -f - | tar -x -C "$work/"
(
  cd "$work"
  git init -q -b main
  git -c user.name=ledger -c user.email=ledger@example.com add -A
  git -c user.name=ledger -c user.email=ledger@example.com commit -q -m "ledger fixture"
)

if [ -n "${GSD_CORE_DIR:-}" ]; then
  real=$(cd "$work" && pwd -P)
  # As record-security.sh: JSON with ISO timestamps, dates, 40-hex hashes and the copy's path replaced.
  normalise() {
    WORK="$work" REAL="$real" node -e '
      let text = "";
      process.stdin.on("data", (c) => (text += c)).on("end", () => {
        const walk = (v) => {
          if (typeof v === "string") {
            return v
              .replace(/\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?Z/g, "<timestamp>")
              .replace(/\b[0-9a-f]{40}\b/g, "<sha>")
              .split(process.env.REAL).join("<project>")
              .split(process.env.WORK).join("<project>");
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
    node "$tools" --cwd "$work" "$@" >"$staged/$name.raw"
    normalise <"$staged/$name.raw" >"$staged/$name.json"
    rm "$staged/$name.raw"
  }
  record roadmap-analyze roadmap analyze
  record progress-json progress json
  record state-json state json
  record validate-health validate health
fi

command -v cargo-nextest >/dev/null || { echo "record-ledger.sh: cargo-nextest is not on PATH (cargo install cargo-nextest --locked)" >&2; exit 1; }

# Runs the copy's tests with nextest as `@rust/test` does (its profile, one thread so the cases come in one order) into
# `$staged/$1`, with the run's id, timestamps, timings and thread ids normalised. A failing test is the point of the
# red run, so nextest's exit status is not the script's; a missing report is.
tests() {
  config="$work.nextest.toml"
  printf '[profile.specforge]\nfail-fast = false\n\n[profile.specforge.junit]\npath = "%s"\nstore-success-output = false\nstore-failure-output = false\n' \
    "$work.junit.xml" >"$config"
  rm -f "$work.junit.xml"
  (cd "$work" && CARGO_TARGET_DIR="$work.target" cargo-nextest nextest run --locked --workspace --test-threads 1 \
    --tool-config-file "specforge:$config" --profile specforge >/dev/null 2>&1) || true
  [ -f "$work.junit.xml" ] || { echo "record-ledger.sh: nextest wrote no report" >&2; exit 1; }
  sed -E -e 's/uuid="[^"]*"/uuid="00000000-0000-0000-0000-000000000000"/' \
    -e 's/timestamp="[^"]*"/timestamp="2026-11-05T16:40:00.000+00:00"/g' -e 's/time="[^"]*"/time="0"/g' \
    -e 's/(&apos;[a-z0-9_]+&apos;) \([0-9]+\) panicked/\1 panicked/g' "$work.junit.xml" >"$staged/$1"
  rm -f "$config" "$work.junit.xml"
}
tests rust-test.xml
sed -e 's/let mut sum = 0i32;/let mut sum = 0i64;/' -e 's/sum.wrapping_add(e.cents as i32)/sum.wrapping_add(e.cents)/' \
  "$work/src/lib.rs" >"$work/lib.rs.fixed"
cmp -s "$work/src/lib.rs" "$work/lib.rs.fixed" && { echo "record-ledger.sh: the i32 fold is not in src/lib.rs" >&2; exit 1; }
cp "$work/src/lib.rs" "$work/lib.rs.red"
mv "$work/lib.rs.fixed" "$work/src/lib.rs"
tests rust-test-before-quick.xml
mv "$work/lib.rs.red" "$work/src/lib.rs"
grep -q '<failure' "$staged/rust-test.xml" || { echo "record-ledger.sh: the red run has no failing test" >&2; exit 1; }
if grep -q '<failure' "$staged/rust-test-before-quick.xml"; then
  echo "record-ledger.sh: the run before the quick task fails" >&2
  exit 1
fi

mkdir -p "$out"
cp "$staged"/* "$out"/
echo "recorded $(ls "$staged" | wc -l | tr -d ' ') goldens in $out"
