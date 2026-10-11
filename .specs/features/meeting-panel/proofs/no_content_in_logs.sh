#!/usr/bin/env bash
set -euo pipefail
exec python3 "$(dirname "$0")/no_content_in_logs.py"
