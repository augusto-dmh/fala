#!/usr/bin/env bash
# Cases for scripts/ci/install-gitleaks.sh. Usage: test-install-gitleaks.sh [-k <case>]
# installs_pinned_version downloads from github.com; the other cases are offline.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
install="$here/install-gitleaks.sh"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

fail() { echo "FAIL $case: $*" >&2; exit 1; }

installs_pinned_version() {
  local dest="$work/pinned"
  "$install" "$dest" >/dev/null || fail "installer exited $?"
  [ "$("$dest/gitleaks" version)" = "8.30.1" ] || fail "version is not 8.30.1"
}

pins_literal() {
  grep -qx 'GITLEAKS_VERSION=8.30.1' "$install" || fail "version pin missing"
  grep -qx 'GITLEAKS_SHA256=551f6fc83ea457d62a0d98237cbad105af8d557003051f41f3e7ca7b3f2470eb' "$install" \
    || fail "sha256 pin missing"
}

hash_mismatch_exits_1_installs_nothing() {
  local dest="$work/mismatch" base="$work/base"
  mkdir -p "$dest" "$base"
  echo "not the real tarball" >"$base/gitleaks_8.30.1_linux_x64.tar.gz"
  local status=0
  GITLEAKS_BASE_URL="file://$base" "$install" "$dest" >/dev/null 2>&1 || status=$?
  [ "$status" -eq 1 ] || fail "exit $status, want 1"
  [ -z "$(ls -A "$dest")" ] || fail "destination not empty: $(ls "$dest")"
}

unsupported_platform_exits_2() {
  local dest="$work/unsupported" shim="$work/shim"
  mkdir -p "$dest" "$shim"
  printf '#!/bin/sh\necho Darwin\n' >"$shim/uname"
  chmod +x "$shim/uname"
  local status=0
  PATH="$shim:$PATH" GITLEAKS_BASE_URL="file://$work/does-not-exist" "$install" "$dest" >/dev/null 2>&1 || status=$?
  [ "$status" -eq 2 ] || fail "exit $status, want 2"
  [ -z "$(ls -A "$dest")" ] || fail "destination not empty"
}

cases=(installs_pinned_version pins_literal hash_mismatch_exits_1_installs_nothing unsupported_platform_exits_2)
if [ "${1:-}" = "-k" ]; then cases=("${2:?usage: -k <case>}"); fi
for case in "${cases[@]}"; do
  declare -F "$case" >/dev/null || { echo "unknown case: $case" >&2; exit 1; }
  "$case"
  echo "ok $case"
done
