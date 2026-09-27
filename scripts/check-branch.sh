#!/usr/bin/env bash
# Never commit on main; branch names are <type>/<slug> with the commit type vocabulary.
set -euo pipefail

branch=$(git rev-parse --abbrev-ref HEAD)
pattern='^(build|chore|ci|docs|feat|fix|perf|refactor|revert|style|test)/([1-9][0-9]*-)?[a-z0-9]+(-[a-z0-9]+)*$'

if [ "$branch" = "main" ]; then
  echo "error: do not commit on main; create a branch such as feat/short-slug" >&2
  exit 1
fi
if [ "$branch" = "HEAD" ]; then
  exit 0 # detached HEAD (rebase in progress)
fi
if ! [[ "$branch" =~ $pattern ]]; then
  echo "error: branch '$branch' is not <type>/<slug> (types: build chore ci docs feat fix perf refactor revert style test; slug: lowercase kebab-case, optional issue number prefix)" >&2
  exit 1
fi
