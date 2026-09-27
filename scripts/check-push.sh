#!/usr/bin/env bash
# Refuse any push that updates main directly; main only changes through squash-merged PRs.
# Reads the pre-push stdin lines: <local ref> <local sha> <remote ref> <remote sha>.
set -euo pipefail

while read -r _local_ref _local_sha remote_ref _remote_sha; do
  if [ "$remote_ref" = "refs/heads/main" ]; then
    echo "error: direct push to main refused; open a pull request" >&2
    exit 1
  fi
done
exit 0
