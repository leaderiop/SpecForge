#!/usr/bin/env bash
# Local verification gate. CI (.github/workflows/ci.yml) stays the source of
# truth; this only makes the local loop fast.
#
#   scripts/gate.sh          quick: only what the working tree changed
#   scripts/gate.sh full     everything, once per phase
#
# Full gate: nextest (not `cargo test`) for the workspace, doctests only for
# crates that have one, `specforge collect --no-run` + `analyze` on the
# reports the tests just wrote, the extensions' native tests and wasm clippy
# in one shared target dir, workspace clippy, fmt, blob freshness, and
# `specforge check` on every spec corpus. It uses target/debug/specforge (no
# release build). Light steps run alongside the heavy ones; the two heavy
# compiles (nextest build, clippy) never overlap.
set -uo pipefail

MODE=${1:-quick}
ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT"
LOGS=target/gate
EXT_TARGET="$ROOT/target/ext"
SPECFORGE=target/debug/specforge
CORPORA=(spec integrations/rust/spec examples/todo-app)
mkdir -p "$LOGS"
rm -f "$LOGS"/*.status

# step NAME CMD... : run CMD, log to target/gate/NAME.log, record status+time.
step() {
    local name=$1; shift
    local start=$SECONDS
    "$@" >"$LOGS/$name.log" 2>&1 </dev/null
    local rc=$?
    echo "$rc $((SECONDS - start))" >"$LOGS/$name.status"
    return $rc
}

# Crates with at least one runnable doctest (a ``` fence that isn't
# text/ignore/json/...): rustdoc costs ~15 s per crate even with none.
doctest_packages() {
    local dir
    for dir in crates/* integrations/rust/*; do
        [ -f "$dir/Cargo.toml" ] && [ -d "$dir/src" ] || continue
        if grep -rhE '^\s*//[/!]\s*```' "$dir/src" | awk '
            { sub(/^[ \t]*\/\/[\/!][ \t]*```/, ""); lang = $0 }
            open { open = 0; next }
            { open = 1; if (lang ~ /^(rust|compile_fail|should_panic|no_run)?[ \t]*$/) found = 1 }
            END { exit !found }'; then
            sed -n 's/^name *= *"\(.*\)"/\1/p' "$dir/Cargo.toml" | head -1
        fi
    done
}

extensions_check() {
    local ext status=0
    for ext in "$@"; do
        echo "== $ext"
        CARGO_TARGET_DIR="$EXT_TARGET" cargo test -q --manifest-path "extensions/$ext/Cargo.toml" || status=1
        CARGO_TARGET_DIR="$EXT_TARGET" cargo clippy -q --manifest-path "extensions/$ext/Cargo.toml" \
            --target wasm32-wasip2 -- -D warnings || status=1
    done
    return $status
}

# Same check as CI: the workspace. (Extension crates are outside it.)
fmt_check() {
    cargo fmt --all -- --check
}

spec_checks() {
    local corpus status=0
    local summary
    for corpus in "${CORPORA[@]}"; do
        summary=$("$SPECFORGE" check "$corpus" 2>&1 | tail -1)
        echo "$corpus: $summary"
        case $summary in "0 errors, 0 warnings"*) ;; *) status=1 ;; esac
    done
    return $status
}

# Dogfood: record what the nextest run just proved, then require no
# unknown entities (W115), no failing proof (A014), no undeclared
# obligation (A016).
dogfood() {
    "$SPECFORGE" collect --no-run --format json >"$LOGS/collect.json" || return 1
    if grep -q '"W115"' "$LOGS/collect.json"; then
        echo "W115 in collect:"; grep -A2 '"W115"' "$LOGS/collect.json"; return 1
    fi
    "$SPECFORGE" analyze coverage --json >"$LOGS/analyze.json"
    python3 - "$LOGS/analyze.json" <<'PY'
import json, sys
doc = json.load(open(sys.argv[1]))
bad = [f for p in doc["passes"] for f in p["findings"] if f["code"] in ("A014", "A016")]
for f in bad:
    print(f["code"], f["message"])
funnel = next(p for p in doc["passes"] if p["pass"].endswith(":coverage"))["summary"]["test_results"]
print("obligations proven:", funnel and funnel.get("obligations_proven"))
sys.exit(1 if bad else 0)
PY
}

report() {
    local failed=0 f name rc secs
    printf '\n%-14s %6s  %s\n' STEP TIME RESULT
    for f in "$LOGS"/*.status; do
        [ -e "$f" ] || continue
        name=$(basename "$f" .status)
        read -r rc secs <"$f"
        if [ "$rc" = 0 ]; then
            printf '%-14s %5ss  ok\n' "$name" "$secs"
        else
            printf '%-14s %5ss  FAILED (see %s/%s.log)\n' "$name" "$secs" "$LOGS" "$name"
            failed=1
        fi
    done
    printf '%-14s %5ss\n' total "$SECONDS"
    return $failed
}

run_full() {
    # Light, independent of the workspace build: in the background.
    step fmt fmt_check &
    step builtins cargo run -q -p xtask --bin build-builtins -- --check &
    local exts=()
    for m in extensions/*/Cargo.toml; do exts+=("$(basename "$(dirname "$m")")"); done
    step extensions extensions_check "${exts[@]}" &

    # Heavy compile #1: every test binary plus the CLI.
    step build cargo nextest run --workspace --no-run --cargo-quiet || { wait; report; return 1; }
    # Tests mostly wait on I/O once built, so clippy (heavy compile #2)
    # runs alongside them; doctests and spec checks are light.
    step nextest cargo nextest run --workspace --profile gate &
    step clippy cargo clippy -q --workspace --all-targets -- -D warnings &
    local docs
    docs=$(doctest_packages | sed 's/^/-p /' | tr '\n' ' ')
    if [ -n "$docs" ]; then
        # shellcheck disable=SC2086
        step doctests cargo test -q --doc $docs &
    fi
    step specs spec_checks &
    wait
    if [ "$(cut -d' ' -f1 "$LOGS/nextest.status")" = 0 ]; then
        step dogfood dogfood
    fi
    report
}

run_quick() {
    local changed pkgs=() exts=() specs=0 builtins=0 file dir name
    changed=$( { git diff --name-only HEAD; git ls-files --others --exclude-standard; } | sort -u)
    [ -n "$changed" ] || { echo "nothing changed"; return 0; }
    for file in $changed; do
        case $file in
            crates/*|integrations/rust/specforge-*|xtask/*)
                dir=$(echo "$file" | cut -d/ -f1-2)
                [[ $file == integrations/* ]] && dir=$(echo "$file" | cut -d/ -f1-3)
                [[ $file == xtask/* ]] && dir=xtask
                name=$(sed -n 's/^name *= *"\(.*\)"/\1/p' "$dir/Cargo.toml" 2>/dev/null | head -1)
                [ -n "$name" ] && pkgs+=("$name")
                [[ $dir == crates/specforge-extension-sdk* ]] && builtins=1
                ;;
            extensions/*)
                exts+=("$(echo "$file" | cut -d/ -f2)"); builtins=1 ;;
        esac
        case $file in
            *.spec|spec/*|examples/*|integrations/rust/spec/*) specs=1 ;;
        esac
    done
    # Deduplicate (no mapfile: macOS ships bash 3.2).
    local unique=()
    while IFS= read -r name; do [ -n "$name" ] && unique+=("$name"); done \
        < <(printf '%s\n' "${pkgs[@]+"${pkgs[@]}"}" | sort -u)
    pkgs=("${unique[@]+"${unique[@]}"}")
    unique=()
    while IFS= read -r name; do [ -n "$name" ] && unique+=("$name"); done \
        < <(printf '%s\n' "${exts[@]+"${exts[@]}"}" | sort -u)
    exts=("${unique[@]+"${unique[@]}"}")
    echo "crates: ${pkgs[*]:-none}; extensions: ${exts[*]:-none}; specs: $specs"

    step fmt fmt_check &
    [ "$builtins" = 1 ] && step builtins cargo run -q -p xtask --bin build-builtins -- --check &
    [ ${#exts[@]} -gt 0 ] && step extensions extensions_check "${exts[@]}" &
    if [ ${#pkgs[@]} -gt 0 ]; then
        # Build and lint the whole workspace (cached, and the same feature
        # set as the full gate: `-p` would rebuild dependencies with other
        # features), but run only the changed crates' tests.
        local filter
        filter=$(printf 'package(=%s) | ' "${pkgs[@]}")
        step nextest cargo nextest run --workspace --profile quick -E "${filter% | }"
        step clippy cargo clippy -q --workspace --all-targets -- -D warnings
        if printf '%s\n' "${pkgs[@]}" | grep -qxF -f <(doctest_packages); then
            # shellcheck disable=SC2046
            step doctests cargo test -q --doc $(printf '%s\n' "${pkgs[@]}" | grep -xF -f <(doctest_packages) | sed 's/^/-p /')
        fi
    fi
    if [ "$specs" = 1 ]; then
        cargo build -q -p specforge-cli && step specs spec_checks
    fi
    wait
    report
}

case $MODE in
    full) run_full ;;
    quick) run_quick ;;
    *) echo "usage: $0 [quick|full]" >&2; exit 2 ;;
esac
