#!/usr/bin/env bash
# The PR body becomes the squash commit body: four sections with real content, no template
# comments, no TODOs, no generated footers. Reads $BODY (set by CI).
set -euo pipefail

body=${BODY:?BODY must contain the pull request body}
status=0

for section in "Problem" "Change" "Verification" "AI assistance"; do
  content=$(awk -v h="## $section" '
    $0 == h { inside = 1; next }
    /^## / { inside = 0 }
    inside && !/^<!--/ && NF { print }
  ' <<<"$body")
  if [ -z "$content" ]; then
    echo "error: section '## $section' is missing or empty" >&2
    status=1
  fi
done

check() { # <regex> <reason>
  if grep -qiE "$1" <<<"$body"; then
    echo "error: PR body $2" >&2
    status=1
  fi
}
check '<!--' 'still contains template comments'
check '\bTODO\b' 'contains a TODO'
check 'Co-Authored-By:' 'contains Co-Authored-By'
check 'Generated with' 'contains a "Generated with" footer'
check 'Claude-Session:' 'contains a session link'
check '\.specs/' 'references a local spec path'

exit "$status"
