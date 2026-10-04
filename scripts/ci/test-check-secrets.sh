#!/usr/bin/env bash
# Self-test for scripts/check-secrets.sh: proves the scanner flags a key and passes a clean repo.
# Usage: test-check-secrets.sh [-k <case>]
# CHECK_SECRETS overrides the script under test (blind_scanner_fails_selftest uses it).
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
check=${CHECK_SECRETS:-$here/../check-secrets.sh}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

fail() { echo "FAIL $case: $*" >&2; exit 1; }

# The key is built at run time so no key-shaped literal lives in the repo.
key="AIza$(python3 -c 'import secrets, string; a = string.ascii_letters + string.digits + "_-"; print("".join(secrets.choice(a) for _ in range(35)))')"

new_repo() {
  local dir
  dir=$(mktemp -d "$work/$1.XXXXXX")
  git -C "$dir" init -q
  echo "$2" >"$dir/$3"
  git -C "$dir" add .
  git -C "$dir" -c user.name=selftest -c user.email=selftest@example.invalid commit -q -m selftest >/dev/null
  echo "$dir"
}

leaky() { new_repo leaky "GEMINI_API_KEY=\"$key\"" gemini.env; }
clean() { new_repo clean "nothing secret here" notes.txt; }

# Runs the script in a repo; sets $status and $output (stdout and stderr together).
scan() {
  status=0
  output=$(cd "$1" && "$check" 2>&1) || status=$?
}

leaky_repo_exits_1_clean_repo_exits_0() {
  scan "$(leaky)"
  [ "$status" -eq 1 ] || fail "leaky repo exit $status, want 1"
  scan "$(clean)"
  [ "$status" -eq 0 ] || fail "clean repo exit $status, want 0: $output"
}

leak_output_names_rule_file_line_without_secret() {
  scan "$(leaky)"
  [ "$status" -eq 1 ] || fail "exit $status, want 1"
  grep -Eq 'RuleID: +[a-z0-9-]+' <<<"$output" || fail "no rule name in: $output"
  grep -Eq 'File: +gemini\.env' <<<"$output" || fail "no file name in: $output"
  grep -Eq 'Line: +1$' <<<"$output" || fail "no line number in: $output"
  if grep -qF "$key" <<<"$output"; then fail "the key appears in the output"; fi
}

clean_repo_prints_ok() {
  scan "$(clean)"
  [ "$status" -eq 0 ] || fail "exit $status, want 0"
  grep -qx 'ok: no secrets found' <<<"$output" || fail "no ok line in: $output"
}

missing_gitleaks_exits_2() {
  local empty="$work/empty-path"
  mkdir -p "$empty"
  status=0
  output=$(cd "$(clean)" && PATH="$empty" /bin/bash "$check" 2>&1) || status=$?
  [ "$status" -eq 2 ] || fail "exit $status, want 2"
  grep -q 'scripts/ci/install-gitleaks.sh' <<<"$output" || fail "no install hint in: $output"
}

blind_scanner_fails_selftest() {
  printf '#!/bin/sh\nexit 0\n' >"$work/blind.sh"
  chmod +x "$work/blind.sh"
  status=0
  CHECK_SECRETS="$work/blind.sh" "$0" -k leaky_repo_exits_1_clean_repo_exits_0 >/dev/null 2>&1 || status=$?
  [ "$status" -eq 1 ] || fail "self-test exit $status with a blind scanner, want 1"
}

cases=(leaky_repo_exits_1_clean_repo_exits_0 leak_output_names_rule_file_line_without_secret
  clean_repo_prints_ok missing_gitleaks_exits_2 blind_scanner_fails_selftest)
if [ "${1:-}" = "-k" ]; then cases=("${2:?usage: -k <case>}"); fi
for case in "${cases[@]}"; do
  declare -F "$case" >/dev/null || { echo "unknown case: $case" >&2; exit 1; }
  "$case"
  echo "ok $case"
done
