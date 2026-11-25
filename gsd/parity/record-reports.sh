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
# standard output, so no volume is created or mounted and the fixture itself is never touched. The tools are installed
# in the container at the exact versions below and print their versions to standard error. The reports are replaced
# only when every step of every named fixture succeeded; a step that fails fails the script.
#
# Recorded, per fixture:
#   taskboard, vault  javascript-coverage.lcov  c8 around the fixture's `npm test`, the `@javascript/coverage` argv
#
# Normalised, so a re-run on the same tools gives the same bytes: a leading `SF:/project/` in an LCOV becomes `SF:`
# (a path outside the project is W203 to the pack; c8 writes relative paths already).
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)

NODE_IMAGE=node:26-bookworm-slim@sha256:0042e17d232a87a2cd485be1da7900fca4d10a4ad11b29e9285da1d2e54798c9
C8=12.0.0

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
    docker run --rm -i --platform linux/amd64 --workdir /project "$image" sh -euc \
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

fixtures=("$@")
[ ${#fixtures[@]} -gt 0 ] || fixtures=(taskboard vault)
for fixture in "${fixtures[@]}"; do
  case $fixture in
    taskboard | vault) "$fixture" ;;
    *) echo "record-reports.sh: no fixture '$fixture' (taskboard, vault)" >&2; exit 1 ;;
  esac
done

for fixture in "${fixtures[@]}"; do
  out="$here/goldens/$fixture/reports"
  mkdir -p "$out"
  cp "$staged/$fixture"/* "$out"/
  echo "recorded $(ls "$staged/$fixture" | wc -l | tr -d ' ') reports in $out"
done
