#!/usr/bin/env bash
# Invariant (ADR-0002): nothing in crates/ depends on tauri, directly or transitively.
set -euo pipefail
cd "$(dirname "$0")/.."

status=0
if grep -nE '^[[:space:]]*(\[[a-z.-]*dependencies\.)?"?tauri' crates/*/Cargo.toml; then
  echo "error: a Cargo.toml in crates/ lists a tauri dependency" >&2
  status=1
fi

for manifest in crates/*/Cargo.toml; do
  pkg=$(sed -n 's/^name = "\(.*\)"$/\1/p' "$manifest" | head -n1)
  if cargo tree -p "$pkg" --target all -e normal,build --prefix none --format '{p}' | grep -E '^tauri'; then
    echo "error: $pkg depends on tauri transitively" >&2
    status=1
  fi
done

[ "$status" -eq 0 ] && echo "ok: no tauri in crates/"
exit "$status"
