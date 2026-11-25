#!/usr/bin/env bash
# Records the tool reports the parity fixtures' report packs read into goldens/<fixture>/reports/, for `cargo gate`'s
# dogfood check, which collects each fixture with every pack it detects from these files (ADR 0393 D9). Run by hand on
# a machine with Docker, never in CI:
#
#   gsd/parity/record-reports.sh                     # every fixture
#   gsd/parity/record-reports.sh ledger              # one or more of: taskboard vault ledger
#
# Each tool runs in a `--platform linux/amd64` container of an image pinned by digest, on a copy of the fixture's
# tracked files at /project: the copy goes in as a tar stream on standard input and the reports come out as a tar on
# standard output, so no volume is created or mounted and the fixture itself is never touched. A fixture holding an
# uncommitted or untracked file is refused before anything runs: its reports would be ones the committed tree cannot
# reproduce. The tools are installed in the container at the exact versions below and print their versions to standard
# error; only c8 itself is pinned, npm resolves its dependency tree at record time. Each named fixture's reports
# directory is replaced as a whole (a report no longer produced goes) only when every step of every named fixture
# succeeded; a step that fails fails the script. osv-scanner sends the ledger lockfile's package names (`ledger 0.1.0`,
# it has no dependency) to osv.dev; nothing else leaves the machine but the images' and tools' downloads.
#
# Recorded, each with the pack's own argv:
#   taskboard, vault  javascript-coverage.lcov  c8 around the fixture's `npm test` (`@javascript/coverage`)
#   ledger            rust-coverage.lcov        cargo llvm-cov over nextest (`@rust/coverage`; the ledger has a test
#                                               that fails by design, which `--ignore-run-fail` lets through)
#                     rust-lint.jsonl           cargo clippy's JSON messages (`@rust/lint`)
#                     rust-sbom.cdx.json        cargo cyclonedx (`@rust/deps`, rust-sbom)
#                     rust-osv.json             osv-scanner over Cargo.lock (`@rust/deps`, rust-osv)
#                     rust-api.txt              cargo public-api on its pinned nightly (`@rust/api`)
#
# Normalised, so a re-run on the same tools gives the same bytes: a leading `SF:/project/` in an LCOV becomes `SF:`
# (a path outside the project is W203 to the pack; c8 writes relative paths already), and the SBOM's `serialNumber`
# and `timestamp` become `urn:uuid:00000000-0000-0000-0000-000000000000` and `1970-01-01T00:00:00Z`.
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)

NODE_IMAGE=node:26-bookworm-slim@sha256:0042e17d232a87a2cd485be1da7900fca4d10a4ad11b29e9285da1d2e54798c9
C8=12.0.0
RUST_IMAGE=rust:1.99.1@sha256:8a4e90c7946b74626488b1446f712c0fe0ed1a596cbd3d871684bc9c5433ae07
# The nightly cargo public-api 0.52 builds rustdoc JSON with (its README's compatibility matrix).
NIGHTLY=nightly-2025-11-22
LLVM_COV=0.9.2
NEXTEST=0.9.146
CYCLONEDX=0.5.10
PUBLIC_API=0.52.0
OSV_IMAGE=ghcr.io/google/osv-scanner:v2.7.1@sha256:a70ffffc2bfd33482953e5d6b64c6fcdb30239717a07b0f0d3ab2859ce3e4cde

# Everything the script writes but the reports lives in one directory, removed on exit.
root=$(mktemp -d)
staged="$root/staged"
trap 'rm -rf "$root"' EXIT
# git never reads the operator's configuration or runs their hooks, as seed.py.
quiet_git() {
  GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1 git -c core.hooksPath=/dev/null "$@"
}
# The fixture $1's tracked files as a tar on standard output (no macOS metadata: COPYFILE_DISABLE).
tracked() {
  quiet_git -C "$here/fixtures/$1" ls-files -z --cached --others --exclude-standard |
    COPYFILE_DISABLE=1 tar -C "$here/fixtures/$1" -c --null -T - -f -
}
# Runs the script $2 with `sh -euc` in a fresh container of image $1 holding the fixture $3's copy at /project; the
# script's standard output is a tar of the reports, unpacked into the fixture's staged directory. Further arguments are
# the script's `$1`...
container() {
  image=$1 script=$2 fixture=$3
  shift 3
  mkdir -p "$staged/$fixture"
  tracked "$fixture" |
    docker run --rm -i --platform linux/amd64 --workdir /project --entrypoint sh "$image" -euc \
      "tar -x --no-same-owner; $script" sh "$@" |
    tar -x -C "$staged/$fixture"
}

# taskboard, vault: c8 around the fixture's own `npm test`, exactly `@javascript/coverage`'s argv. The tests pass, so
# c8's exit status (the test command's) is the script's.
javascript() {
  container "$NODE_IMAGE" '
    echo "node $(node --version)" >&2
    npm install --no-save --no-audit --no-fund --no-update-notifier --loglevel=error "c8@$1" >&2
    npm ls --depth=0 c8 >&2
    npx --no -- c8 --reporter=lcovonly --reports-dir=.specforge/reports/javascript-coverage npm test >&2
    sed "s|^SF:/project/|SF:|" .specforge/reports/javascript-coverage/lcov.info >javascript-coverage.lcov
    tar -c javascript-coverage.lcov
  ' "$1" "$C8"
}

taskboard() { javascript taskboard; }
vault() { javascript vault; }

# ledger: the Rust tools in one container (installed with --locked; the pinned nightly is active for cargo public-api
# only, every other tool runs on the image's stable), then osv-scanner in its own image.
ledger() {
  container "$RUST_IMAGE" '
    rustup component add llvm-tools-preview clippy >&2
    rustup toolchain install "$1" --profile minimal >&2
    cargo install --locked "cargo-llvm-cov@$2" "cargo-nextest@$3" "cargo-cyclonedx@$4" "cargo-public-api@$5" >&2
    for tool in "rustc --version" "rustc +$1 --version" "cargo clippy --version" "cargo llvm-cov --version" \
      "cargo nextest --version" "cargo cyclonedx --version" "cargo public-api --version"; do
      $tool >&2
    done
    mkdir /reports
    cargo llvm-cov nextest --workspace --ignore-run-fail --lcov --output-path /reports/lcov.info >&2
    sed "s|^SF:/project/|SF:|" /reports/lcov.info >/reports/rust-coverage.lcov
    cargo clippy --workspace --all-targets --message-format=json >/reports/rust-lint.jsonl
    cargo cyclonedx --format json --spec-version 1.5 --all-features --override-filename rust-sbom.cdx >&2
    sed -e "s|\"serialNumber\": *\"[^\"]*\"|\"serialNumber\": \"urn:uuid:00000000-0000-0000-0000-000000000000\"|" \
      -e "s|\"timestamp\": *\"[^\"]*\"|\"timestamp\": \"1970-01-01T00:00:00Z\"|" rust-sbom.cdx.json \
      >/reports/rust-sbom.cdx.json
    # cargo public-api finds a toolchain named `nightly` by itself; a dated one must be the active toolchain.
    RUSTUP_TOOLCHAIN="$1" cargo public-api --color never -ss >/reports/rust-api.txt
    cd /reports
    tar -c rust-coverage.lcov rust-lint.jsonl rust-sbom.cdx.json rust-api.txt
  ' ledger "$NIGHTLY" "$LLVM_COV" "$NEXTEST" "$CYCLONEDX" "$PUBLIC_API"
  # Exit 1 is "vulnerabilities found", a report like any other; every other non-zero is a failure.
  container "$OSV_IMAGE" '
    /osv-scanner --version >&2
    status=0
    /osv-scanner scan source --lockfile Cargo.lock --format json --output-file rust-osv.json >&2 || status=$?
    [ "$status" -le 1 ] || { echo "record-reports.sh: osv-scanner exited $status" >&2; exit 1; }
    tar -c rust-osv.json
  ' ledger
}

fixtures=("$@")
[ ${#fixtures[@]} -gt 0 ] || fixtures=(taskboard vault ledger)
for fixture in "${fixtures[@]}"; do
  case $fixture in
    taskboard | vault | ledger) ;;
    *) echo "record-reports.sh: no fixture '$fixture' (taskboard, vault, ledger)" >&2; exit 1 ;;
  esac
  if [ -n "$(quiet_git -C "$here/fixtures/$fixture" status --porcelain -- .)" ]; then
    echo "record-reports.sh: fixtures/$fixture holds uncommitted or untracked files; commit or remove them" >&2
    exit 1
  fi
done
for fixture in "${fixtures[@]}"; do
  "$fixture"
done

# A container's tar may hold a symlink (or a device, a FIFO, a directory) that a copy would follow out to a host file
# or trip on halfway: the reports are regular files directly in staged/<fixture>, so anything else refuses every
# fixture before a reports directory is touched.
odd=$(find "$staged" -mindepth 2 ! -type f)
if [ -n "$odd" ]; then
  echo "record-reports.sh: a container wrote something other than a regular report file, nothing recorded:" >&2
  echo "$odd" >&2
  exit 1
fi
for fixture in "${fixtures[@]}"; do
  out="$here/goldens/$fixture/reports"
  rm -rf "$out"
  mkdir -p "$out"
  cp -P "$staged/$fixture"/* "$out"/
  echo "recorded $(ls "$staged/$fixture" | wc -l | tr -d ' ') reports in $out"
done
