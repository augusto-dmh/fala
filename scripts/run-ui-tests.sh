#!/usr/bin/env bash
# Runs every frontend test script (`src/**/*.test.ts(x)`). Each one is a plain Bun script that
# prints `<check> ok` and exits non-zero on the first failed assertion, so a new test file is
# picked up without being listed anywhere.
set -euo pipefail
cd "$(dirname "$0")/.."

failed=()
count=0
while IFS= read -r file; do
  count=$((count + 1))
  if ! bun "$file" >/dev/null 2>&1; then
    failed+=("$file")
  fi
done < <(git ls-files 'src/**/*.test.ts' 'src/**/*.test.tsx')

if [ "${#failed[@]}" -gt 0 ]; then
  for file in "${failed[@]}"; do
    echo "::group::$file"
    bun "$file" || true
    echo "::endgroup::"
  done
  echo "error: ${#failed[@]} of $count frontend test scripts failed: ${failed[*]}" >&2
  exit 1
fi
echo "$count frontend test scripts passed"
