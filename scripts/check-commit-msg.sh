#!/usr/bin/env bash
# Commit messages reference only what lives in the repo and carry no generated footers.
# AI attribution is the Assisted-by: trailer, written by the harness (.claude/settings.json).
set -euo pipefail

file=${1:?usage: check-commit-msg.sh <commit-msg-file>}
msg=$(grep -v '^#' "$file" || true)
status=0

check() { # <regex> <reason>
  if grep -qiE "$1" <<<"$msg"; then
    echo "error: commit message $2" >&2
    status=1
  fi
}

check '^Co-Authored-By:' 'uses Co-Authored-By; AI attribution is the Assisted-by: trailer'
check 'Generated with' 'contains a "Generated with" footer'
check '^Claude-Session:' 'contains a session link'
check '^Signed-off-by:' 'contains Signed-off-by (no DCO here; agents never sign off)'
check '\.specs/' 'references a local spec path'
check '\b(cycle|task|gate) [0-9]+\b' 'references an internal skill task id (ROADMAP phases are fine)'

exit "$status"
