#!/usr/bin/env bash
# No secrets in git history (ADR-0008: keys live in the OS keyring, never in the repo). Scans the
# repository in the current directory with the default gitleaks rules plus .github/gitleaks.toml.
# Findings print rule, file and line with the value redacted. Install: scripts/ci/install-gitleaks.sh
# Exit: 0 clean, 1 secret found, 2 gitleaks missing.
set -euo pipefail

if ! command -v gitleaks >/dev/null 2>&1; then
  echo "error: gitleaks not found in PATH; run scripts/ci/install-gitleaks.sh" >&2
  exit 2
fi

# Builtins only: the self-test runs this with an empty PATH to reach the branch above.
script_dir=$(cd "${BASH_SOURCE[0]%/*}" && pwd)
config="$script_dir/../.github/gitleaks.toml"

if gitleaks git --no-banner --no-color --redact --verbose --config "$config" .; then
  echo "ok: no secrets found"
else
  status=$?
  echo "error: gitleaks found secrets or failed (exit $status); values are redacted above" >&2
  exit 1
fi
