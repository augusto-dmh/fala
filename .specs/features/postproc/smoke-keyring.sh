#!/usr/bin/env bash
# C29: o binário real grava no Secret Service desta máquina (gnome-keyring), sob o service
# br.com.augusto.fala. Usa só o provedor de teste `teste_cli` e um valor que não é chave.
set -euo pipefail

: "${CARGO_TARGET_DIR:=/home/augusto/projects/fala/target}"
export CARGO_TARGET_DIR
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
cargo build -q -p fala-cli
cli="$CARGO_TARGET_DIR/debug/fala-cli"
provider=teste_cli
value=nao-e-chave

trap '"$cli" key delete "$provider" >/dev/null 2>&1 || true' EXIT

printf '%s' "$value" | "$cli" key set "$provider"
stored="$(secret-tool lookup service br.com.augusto.fala username "$provider")"
[ "$stored" = "$value" ] || { echo "FAIL: secret-tool leu '$stored'" >&2; exit 1; }
[ "$("$cli" key status "$provider")" = definida ] || { echo "FAIL: status depois do set" >&2; exit 1; }
"$cli" key delete "$provider"
[ "$("$cli" key status "$provider")" = ausente ] || { echo "FAIL: status depois do delete" >&2; exit 1; }
if secret-tool lookup service br.com.augusto.fala username "$provider" >/dev/null 2>&1; then
  echo "FAIL: a entrada continua no keyring" >&2
  exit 1
fi
echo "ok: set, lookup, status e delete contra o Secret Service"
