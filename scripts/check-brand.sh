#!/usr/bin/env bash
# No Handy branding in code or config. The only allowed references are third-party names and
# upstream provenance, listed in NOTICE.md. Markdown docs are excluded on purpose: README,
# NOTICE, ADRs and the design doc must attribute the fork.
set -euo pipefail
cd "$(dirname "$0")/.."

allowed='blob\.handy\.computer|handy\.computer/docs|handy-computer|handy-keys|handy_keys::|to_handy_string|cjpais/'
hits=$(git grep -niI 'handy' -- . ':!*.md' ':!LICENSE' ':!scripts/check-brand.sh' | grep -viE "$allowed" || true)

if [ -n "$hits" ]; then
  echo "$hits"
  echo "error: Handy branding found outside the allowlist (see NOTICE.md)" >&2
  exit 1
fi
echo "ok: no Handy branding outside the allowlist"
