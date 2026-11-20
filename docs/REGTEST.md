# BITE Stage 2 — Bitcoin Core regtest sandbox

This sandbox is for **offline development and CI smoke tests only**. It starts Bitcoin Core 32.1 on a private regression-test chain, creates a disposable local test wallet, mines enough blocks for coinbase maturity, and verifies the active chain.

## Requirements

- Docker Engine with Docker Compose v2
- Bash
- A machine capable of running the pinned bitcoin/bitcoin:32.1 test image

The bitcoin/bitcoin Docker image is explicitly an **unofficial image intended for testing**, not an image endorsed by the Bitcoin Core project. It must not be used as a production binary source. For non-testing purposes, obtain Bitcoin Core from the official release page and verify the downloaded binaries as instructed by the Bitcoin Core project.

## Run

From the repository root, run:

    ./scripts/regtest-smoke.sh

The script starts the isolated node, verifies that Bitcoin Core reports chain=regtest, creates or loads the bite-stage2-miner test wallet, obtains a regtest-only bcrt1 address, and mines until the chain has at least 101 blocks.

To stop the container but keep local test data:

    docker compose --project-name bite-stage2-regtest --file contrib/regtest/compose.yaml down

To delete the disposable chain and test wallet explicitly:

    docker compose --project-name bite-stage2-regtest --file contrib/regtest/compose.yaml down --volumes

## Isolation and trust boundaries

- The node is forced to -regtest=1; the script independently checks the reported chain before mining.
- Peer listening, outbound peer connections, DNS seeding, discovery and UPnP are disabled.
- The Compose network is marked internal.
- No RPC or P2P port is published to the host.
- RPC is used only by bitcoin-cli inside the same container, using Bitcoin Core's local cookie authentication.
- The test wallet and mined coins have no value and must never be used as real funds.
- The Docker image is pinned to a version tag, not a cryptographically verified image digest. It is only a test fixture; do not rely on it for production.
- This smoke test proves only that an isolated regtest node can start and produce regtest blocks. It does **not** implement BITE transaction construction, PSBT validation, counterparty signing, BTC ownership proofs, matching or settlement.

Bitcoin Core warns that RPC credentials grant significant control over the node and its filesystem and that RPC must not be exposed to the public Internet. This sandbox avoids publishing the RPC port at all.

## Next integration gates

1. Add a read-only Rust RPC adapter using explicit regtest endpoint and chain checks.
2. Verify block hashes, transaction inclusion, confirmations and reorg rollback from the node.
3. Add PSBT parsing and compare every input/output/fee against a reviewed trade intent.
4. Add simulated two-party signing with disposable regtest-only keys.
5. Add adversarial reorg, conflicting-spend, fee-substitution and PSBT-mutation tests.

None of these gates enables mainnet. Mainnet remains a separate release decision after independent audits and jurisdiction-specific legal review.
