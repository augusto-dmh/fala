#!/usr/bin/env bash
# Installs the pinned gitleaks, the secret scanner ADR-0008 asks CI to run. The tarball is checked
# against a sha256 written here, so a retagged or tampered release cannot reach the runner.
# Bump GITLEAKS_VERSION and GITLEAKS_SHA256 together, in a PR of their own.
# Usage: install-gitleaks.sh [dir]   (default: $HOME/.local/bin; prints the installed path)
# Exit: 0 installed, 1 download failed or sha256 differs (nothing installed), 2 not linux x64.
set -euo pipefail

GITLEAKS_VERSION=8.30.1
GITLEAKS_SHA256=551f6fc83ea457d62a0d98237cbad105af8d557003051f41f3e7ca7b3f2470eb

dest=${1:-$HOME/.local/bin}
tarball="gitleaks_${GITLEAKS_VERSION}_linux_x64.tar.gz"
# GITLEAKS_BASE_URL exists so the tests can serve a tarball without the network.
base=${GITLEAKS_BASE_URL:-https://github.com/gitleaks/gitleaks/releases/download/v${GITLEAKS_VERSION}}

if [ "$(uname -s)" != "Linux" ] || [ "$(uname -m)" != "x86_64" ]; then
  echo "error: only linux x64 is supported (got $(uname -s) $(uname -m))" >&2
  exit 2
fi

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

curl --fail --silent --show-error --location --output "$tmp/$tarball" "$base/$tarball" || {
  echo "error: could not download $base/$tarball" >&2
  exit 1
}

actual=$(sha256sum "$tmp/$tarball" | cut -d' ' -f1)
if [ "$actual" != "$GITLEAKS_SHA256" ]; then
  echo "error: sha256 of $tarball is $actual, expected $GITLEAKS_SHA256" >&2
  exit 1
fi

tar -xzf "$tmp/$tarball" -C "$tmp" gitleaks
mkdir -p "$dest"
install -m 0755 "$tmp/gitleaks" "$dest/gitleaks"
echo "$dest/gitleaks"
