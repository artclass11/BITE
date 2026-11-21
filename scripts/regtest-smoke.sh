#!/usr/bin/env bash
set -Eeuo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
COMPOSE_FILE="$ROOT_DIR/contrib/regtest/compose.yaml"
PROJECT_NAME="bite-stage2-regtest"
WALLET_NAME="bite-stage2-miner"
BITE_REGTEST_DATA_DIR="${BITE_REGTEST_DATA_DIR:-$ROOT_DIR/.bite-regtest-data}"
BITE_OBSERVER_TMP_DIR="${BITE_OBSERVER_TMP_DIR:-${RUNNER_TEMP:-${TMPDIR:-/tmp}}/bite-observer-test}"
BITE_RPC_URL="http://127.0.0.1:18443"
export BITE_REGTEST_DATA_DIR
export BITE_PROJECT_NAME="$PROJECT_NAME"
export BITE_COMPOSE_FILE="$COMPOSE_FILE"

if ! command -v docker >/dev/null 2>&1; then
  echo "Docker is required for the BITE regtest smoke test." >&2
  exit 127
fi
if ! docker compose version >/dev/null 2>&1; then
  echo "Docker Compose v2 is required (docker compose)." >&2
  exit 127
fi
if ! command -v jq >/dev/null 2>&1; then
  echo "jq is required to select the test transaction from the regtest block." >&2
  exit 127
fi
if ! command -v cargo >/dev/null 2>&1; then
  echo "Cargo is required to run the Rust Bitcoin observer integration test." >&2
  exit 127
fi

mkdir -p "$BITE_REGTEST_DATA_DIR" "$BITE_OBSERVER_TMP_DIR"
chmod 700 "$BITE_REGTEST_DATA_DIR" "$BITE_OBSERVER_TMP_DIR"

compose() {
  docker compose --project-name "$PROJECT_NAME" --file "$COMPOSE_FILE" "$@"
}

btc_cli() {
  compose exec --no-TTY bitcoind bitcoin-cli -datadir=/home/bitcoin/.bitcoin -rpcconnect=127.0.0.1 -regtest "$@"
}

echo "Starting isolated Bitcoin Core regtest with RPC bound to host loopback only..."
compose up --detach bitcoind

chain_info=""
last_rpc_error=""
for attempt in {1..60}; do
  running_container="$(compose ps --status running --quiet bitcoind)"
  if [[ -z "$running_container" ]]; then
    echo "Bitcoin Core container exited before RPC became available; logs follow:" >&2
    compose logs --no-color bitcoind >&2 || true
    exit 1
  fi
  if chain_info="$(btc_cli getblockchaininfo 2>&1)"; then
    break
  else
    last_rpc_error="$chain_info"
    chain_info=""
  fi
  sleep 2
done

if [[ -z "$chain_info" ]]; then
  echo "Bitcoin Core RPC did not become ready. Last RPC error: $last_rpc_error" >&2
  compose logs --no-color bitcoind >&2 || true
  exit 1
fi
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
if (( blocks < 103 )); then
  btc_cli generatetoaddress "$((103 - blocks))" "$address" >/dev/null
fi

final_info="$(btc_cli getblockchaininfo)"
final_blocks="$(btc_cli getblockcount)"
if ! grep -Eq '"chain"[[:space:]]*:[[:space:]]*"regtest"' <<<"$final_info"; then
  echo "Regtest chain identity changed unexpectedly." >&2
  exit 1
fi
if (( final_blocks < 103 )); then
  echo "Regtest smoke test failed: expected at least 103 blocks, got $final_blocks." >&2
  exit 1
fi

test_height=101
test_hash="$(btc_cli getblockhash "$test_height")"
test_block="$(btc_cli getblock "$test_hash" 1)"
test_txid="$(jq -r '.tx[0] // empty' <<<"$test_block")"
if [[ ! "$test_txid" =~ ^[0-9a-f]{64}$ ]]; then
  echo "Could not extract a transaction ID from the regtest block." >&2
  exit 1
fi

# Copy only the ephemeral cookie into a runner-owned mode-0600 file.
# The cookie itself is never printed to logs.
cookie_contents="$(compose exec --no-TTY bitcoind sh -c 'cat /home/bitcoin/.bitcoin/regtest/.cookie')"
observer_cookie="$BITE_OBSERVER_TMP_DIR/observer.cookie"
(umask 077; printf '%s' "$cookie_contents" > "$observer_cookie")
unset cookie_contents

# This test-only wrapper mutates the disposable regtest node to create a fork.
# The observer library itself exposes read-only methods only.
cli_wrapper="$BITE_OBSERVER_TMP_DIR/bite-bitcoin-cli"
cat > "$cli_wrapper" <<'CLI'
#!/usr/bin/env bash
set -Eeuo pipefail
docker compose --project-name "$BITE_PROJECT_NAME" --file "$BITE_COMPOSE_FILE" \
  exec --no-TTY bitcoind bitcoin-cli \
  -datadir=/home/bitcoin/.bitcoin -rpcconnect=127.0.0.1 -regtest "$@"
CLI
chmod 700 "$cli_wrapper"

echo "Running the Rust RPC integration test (block lookup, transaction confirmations and a real regtest reorg)..."
BITE_RPC_URL="$BITE_RPC_URL" \
BITE_RPC_COOKIE_FILE="$observer_cookie" \
BITE_BITCOIN_CLI="$cli_wrapper" \
BITE_REGTEST_ADDRESS="$address" \
BITE_TEST_BLOCK_HEIGHT="$test_height" \
BITE_TEST_BLOCK_HASH="$test_hash" \
BITE_TEST_TXID="$test_txid" \
cargo test --locked -p bite-bitcoin-observer --test regtest_rpc -- --ignored --nocapture

echo "BITE read-only observer integration passed. chain=regtest blocks=$final_blocks"
echo "Bitcoin Core RPC is bound to 127.0.0.1:18443 only; no RPC interface is reachable from the LAN or Internet."
echo "The disposable regtest data is in: $BITE_REGTEST_DATA_DIR"
echo "The private observer test files are in: $BITE_OBSERVER_TMP_DIR"
echo "Stop the container with: docker compose --project-name $PROJECT_NAME --file $COMPOSE_FILE down"
echo "Remove disposable test data with: docker compose --project-name $PROJECT_NAME --file $COMPOSE_FILE down --volumes && rm -rf '$BITE_REGTEST_DATA_DIR' '$BITE_OBSERVER_TMP_DIR'"
