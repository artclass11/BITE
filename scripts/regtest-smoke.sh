#!/usr/bin/env bash
set -Eeuo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
COMPOSE_FILE="$ROOT_DIR/contrib/regtest/compose.yaml"
PROJECT_NAME="bite-stage2-regtest"
WALLET_NAME="bite-stage2-miner"

if ! command -v docker >/dev/null 2>&1; then
  echo "Docker is required for the BITE regtest smoke test." >&2
  exit 127
fi

if ! docker compose version >/dev/null 2>&1; then
  echo "Docker Compose v2 is required (docker compose)." >&2
  exit 127
fi

compose() {
  docker compose --project-name "$PROJECT_NAME" --file "$COMPOSE_FILE" "$@"
}

btc_cli() {
  compose exec --no-TTY bitcoind bitcoin-cli -regtest "$@"
}

echo "Starting an isolated Bitcoin Core regtest node..."
compose up --detach --wait --wait-timeout 120 bitcoind

chain_info="$(btc_cli getblockchaininfo)"
if ! grep -Eq '"chain"[[:space:]]*:[[:space:]]*"regtest"' <<<"$chain_info"; then
  echo "Refusing to continue: node did not report chain=regtest." >&2
  exit 1
fi

wallet_dir="$(btc_cli listwalletdir)"
loaded_wallets="$(btc_cli listwallets)"
if ! grep -Fq "\"$WALLET_NAME\"" <<<"$loaded_wallets"; then
  if grep -Fq "\"$WALLET_NAME\"" <<<"$wallet_dir"; then
    btc_cli loadwallet "$WALLET_NAME" >/dev/null
  else
    btc_cli createwallet "$WALLET_NAME" >/dev/null
  fi
fi

address="$(btc_cli "-rpcwallet=$WALLET_NAME" getnewaddress "" bech32)"
if [[ ! "$address" =~ ^bcrt1 ]]; then
  echo "Refusing to mine: expected a regtest bech32 address." >&2
  exit 1
fi

blocks="$(btc_cli getblockcount)"
if [[ ! "$blocks" =~ ^[0-9]+$ ]]; then
  echo "Bitcoin Core returned an invalid block count." >&2
  exit 1
fi

if (( blocks < 101 )); then
  btc_cli generatetoaddress "$((101 - blocks))" "$address" >/dev/null
fi

final_info="$(btc_cli getblockchaininfo)"
final_blocks="$(btc_cli getblockcount)"
if ! grep -Eq '"chain"[[:space:]]*:[[:space:]]*"regtest"' <<<"$final_info"; then
  echo "Regtest chain identity changed unexpectedly." >&2
  exit 1
fi
if (( final_blocks < 101 )); then
  echo "Regtest smoke test failed: expected at least 101 blocks, got $final_blocks." >&2
  exit 1
fi

printf 'BITE regtest smoke test passed. chain=regtest blocks=%s wallet=%s\n' \
  "$final_blocks" "$WALLET_NAME"
echo "The node has no published RPC/P2P ports and its Docker network is isolated."
echo "Stop it with: docker compose --project-name $PROJECT_NAME --file $COMPOSE_FILE down"
echo "Remove the disposable chain only with: docker compose --project-name $PROJECT_NAME --file $COMPOSE_FILE down --volumes"
