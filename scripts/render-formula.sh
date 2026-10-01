#!/usr/bin/env bash
# Render the Homebrew formula for a release.
#
#   scripts/render-formula.sh <version> <SHA256SUMS>  > specforge.rb
#
# SHA256SUMS is the combined checksum file attached to the GitHub Release.
# Copy the output into the tap repo (leaderiop/homebrew-tap, Formula/specforge.rb).
set -euo pipefail

version="${1:?usage: render-formula.sh <version> <SHA256SUMS>}"
sums="${2:?usage: render-formula.sh <version> <SHA256SUMS>}"

sha() {
  local sum
  sum=$(awk -v f="specforge-$1.tar.gz" '$2 == f { print $1 }' "$sums")
  [ -n "$sum" ] || { echo "no checksum for specforge-$1.tar.gz in $sums" >&2; exit 1; }
  echo "$sum"
}

base="https://github.com/leaderiop/SpecForge/releases/download/v${version}"

cat <<EOF
class Specforge < Formula
  desc "Compile specs into a validated, typed entity graph for AI agents"
  homepage "https://github.com/leaderiop/SpecForge"
  version "${version}"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    on_arm do
      url "${base}/specforge-aarch64-apple-darwin.tar.gz"
      sha256 "$(sha aarch64-apple-darwin)"
    end
    on_intel do
      url "${base}/specforge-x86_64-apple-darwin.tar.gz"
      sha256 "$(sha x86_64-apple-darwin)"
    end
  end

  on_linux do
    on_arm do
      url "${base}/specforge-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "$(sha aarch64-unknown-linux-gnu)"
    end
    on_intel do
      url "${base}/specforge-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "$(sha x86_64-unknown-linux-gnu)"
    end
  end

  def install
    bin.install "specforge"
  end

  def caveats
    <<~EOS
      Formal proofs (@specforge/formal) need the z3 solver on your PATH:
        brew install z3
    EOS
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/specforge --version")
  end
end
EOF
