#!/usr/bin/env bash
# Fail above 1000 changed lines unless the PR carries the large-change label.
# Lockfiles, translations and binary assets do not count.
set -euo pipefail

pr=${1:?usage: check-pr-size.sh <pr-number>}
limit=1000

json=$(gh pr view "$pr" --json labels,files)
if jq -e '.labels[] | select(.name == "large-change")' <<<"$json" >/dev/null; then
  echo "ok: large-change label present; size gate skipped"
  exit 0
fi

lines=$(jq '[.files[]
  | select(.path | test("(^|/)(Cargo|bun)\\.lock$|^src/i18n/locales/|\\.(png|ico|icns|wav|onnx)$") | not)
  | .additions + .deletions] | add // 0' <<<"$json")

if [ "$lines" -gt "$limit" ]; then
  echo "error: $lines changed lines (limit $limit). Split the PR, or add the large-change label with a reason in the body." >&2
  exit 1
fi
echo "ok: $lines changed lines"
