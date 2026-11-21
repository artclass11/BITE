# BITE Stage 2 — Bitcoin Core regtest and Rust observer

This harness runs Bitcoin Core 32.1 on a disposable regression-test chain and exercises the BITE Rust observer against the real node.

## Requirements

- Docker Engine and Docker Compose v2
- Bash, jq and Rust/Cargo
- A machine able to run the pinned bitcoin/bitcoin:32.1 testing image

The bitcoin/bitcoin Docker image is an unofficial testing image, not a production binary source. For non-testing purposes, obtain Bitcoin Core from the official release page and verify the binaries as instructed by the Bitcoin Core project.

## Run

From the repository root:

    ./scripts/regtest-smoke.sh

The script creates a disposable test wallet, mines to at least height 103, reads a block and transaction identifier, then runs the Rust observer integration test. The Rust test verifies the active network, chain tip, block body and ancestry, transaction inclusion and confirmations. It then invalidates the current tip and mines a replacement fork so the observer must report the disconnected and replacement blocks.

The regtest container uses Docker host networking on Linux so the host Rust process can reach RPC directly on 127.0.0.1:18443. Bitcoin Core itself binds RPC only to 127.0.0.1 and allows only 127.0.0.1. No port-forward rule is used. Bitcoin peer listening and outbound peer connections, DNS seeding and discovery remain disabled, so the daemon is not offering a P2P endpoint.

The RPC server inside the container binds to its container interfaces so the Docker port forward can reach it. Cookie authentication is mandatory. The script copies the ephemeral cookie to a runner-owned mode-0600 file for the host Rust process and never prints the cookie.

## Clean up

Stop the node but keep the local test data:

    docker compose --project-name bite-stage2-regtest --file contrib/regtest/compose.yaml down

Remove the disposable chain, wallet, cookie and observer test files:

    docker compose --project-name bite-stage2-regtest --file contrib/regtest/compose.yaml down --volumes
    rm -rf .bite-regtest-data /tmp/bite-observer-test

In CI, the entire temporary data directory is removed in an unconditional cleanup step using runner sudo, because the container entrypoint may chown its bind-mounted datadir to the bitcoin user.

## Observer guarantees and boundaries

- The Rust crate accepts only HTTP loopback RPC URLs and authenticates with Bitcoin Core's cookie file.
- It checks chain identity on each chain query and refuses any chain other than regtest.
- The chain tip hash is cross-checked using getblockhash.
- Block body hash, height, previous-block ancestry and transaction IDs are checked against RPC metadata.
- Transaction lookup requires a specific containing block hash; it does not rely on txindex and does not treat mempool presence as settlement.
- Confirmations are recomputed from the current tip after checking the block is still canonical.
- The in-memory chain tracker reports the common ancestor, removed blocks and replacement blocks for reorgs within its configured window. It fails closed when no common ancestor is found inside that window.
- The tracker is currently in-memory. A process restart requires a fresh chain window; persistent cursor storage, crash-safe rescan and external node trust policy remain future work.
- The observer crate exposes no wallet, signing, mining, broadcast or mutating RPC methods. The test fixture alone uses bitcoin-cli to invalidate a disposable regtest block and mine a replacement fork.
- This integration proves connectivity to one local regtest node. It is not an independent multi-node consensus proof, a production chain-monitoring service, BTC ownership proof or trade settlement.

Bitcoin Core RPC credentials grant substantial control over the node. Keep RPC access local and protected; do not expose it to the public Internet. BITE mainnet support remains disabled.
