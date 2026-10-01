#!/usr/bin/env sh
# Install the latest SpecForge release.
#
#   curl -fsSL https://raw.githubusercontent.com/leaderiop/SpecForge/main/install.sh | sh
#
# Environment:
#   SPECFORGE_VERSION   install a specific version (e.g. 0.1.0) instead of the latest
#   SPECFORGE_BIN_DIR   install directory (default: ~/.local/bin)
set -eu

repo="leaderiop/SpecForge"
bin_dir="${SPECFORGE_BIN_DIR:-$HOME/.local/bin}"

case "$(uname -s)-$(uname -m)" in
  Darwin-arm64)            target="aarch64-apple-darwin" ;;
  Darwin-x86_64)           target="x86_64-apple-darwin" ;;
  Linux-x86_64)            target="x86_64-unknown-linux-gnu" ;;
  Linux-aarch64|Linux-arm64) target="aarch64-unknown-linux-gnu" ;;
  *) echo "unsupported platform: $(uname -s)-$(uname -m)" >&2
     echo "build from source instead: cargo install --git https://github.com/$repo specforge-cli" >&2
     exit 1 ;;
esac

if [ -n "${SPECFORGE_VERSION:-}" ]; then
  tag="v${SPECFORGE_VERSION#v}"
else
  tag=$(curl -fsSL "https://api.github.com/repos/$repo/releases/latest" |
    sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n1)
  [ -n "$tag" ] || { echo "could not determine the latest release" >&2; exit 1; }
fi

archive="specforge-$target.tar.gz"
url="https://github.com/$repo/releases/download/$tag"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

echo "Downloading specforge $tag for $target..."
curl -fsSL "$url/$archive" -o "$tmp/$archive"
curl -fsSL "$url/$archive.sha256" -o "$tmp/$archive.sha256"

expected=$(awk '{ print $1 }' "$tmp/$archive.sha256")
if command -v sha256sum >/dev/null 2>&1; then
  actual=$(sha256sum "$tmp/$archive" | awk '{ print $1 }')
else
  actual=$(shasum -a 256 "$tmp/$archive" | awk '{ print $1 }')
fi
[ "$expected" = "$actual" ] || { echo "checksum mismatch, aborting" >&2; exit 1; }

tar xzf "$tmp/$archive" -C "$tmp"
mkdir -p "$bin_dir"
install -m 755 "$tmp/specforge-$target/specforge" "$bin_dir/specforge"

echo "Installed $("$bin_dir/specforge" --version) to $bin_dir/specforge"
case ":$PATH:" in
  *":$bin_dir:"*) ;;
  *) echo "Add $bin_dir to your PATH to use it." ;;
esac
