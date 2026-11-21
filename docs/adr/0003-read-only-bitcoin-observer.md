# ADR 0003 — Read-only Bitcoin Core observer boundary

- **Status:** Accepted for Stage 2 prototype; not a production node-trust policy
- **Date:** 2026-11-21
- **Scope:** Bitcoin Core regtest reads, block/transaction observations and bounded reorg detection

## Context

The earlier Stage 2 regtest harness proved that a disposable Bitcoin Core node could mine blocks, but the BITE Rust application did not query the node. Chain-state transitions and PSBT-hash state in the existing JavaScript prototype were not evidence that a transaction existed in the Bitcoin active chain.

The previous Rust crate name, bitcoincore-rpc, has been archived. Stage 2 therefore uses the newer corepc-client async client and its cookie-file authentication. The dependency is pinned and recorded in Cargo.lock. The observer is intentionally separate from the order-verification crate.

## Decision

1. Add a separate Rust workspace crate, bite-bitcoin-observer, using corepc-client 0.18.0.
2. The public crate API is read-only. It exposes chain tip, bounded canonical chain window, block observation, transaction-in-block observation and a shallow reorg tracker. It does not expose wallet, signing, mining, invalidation, transaction broadcast or other mutating RPC operations.
3. Connect only through an HTTP loopback endpoint and Bitcoin Core's cookie file. Never embed RPC user/password values in URLs or log cookie content.
4. Require the RPC node's getblockchaininfo response to say chain=regtest. Recheck the active-chain tip hash with getblockhash. The current implementation is not a mainnet-capable observer.
5. Block observation cross-checks the RPC header hash, height, previous block hash and block-body hash, then extracts transaction IDs from the parsed block.
6. Transaction lookup requires an explicit containing block hash. It verifies the returned txid and block hash, rejects mempool/unconfirmed/inactive-chain results, confirms the transaction is present in the parsed block, and recomputes confirmations after checking that the block remains active.
7. ChainTracker retains a bounded in-memory window of canonical block identities. It reports initialization, unchanged tip, extension or reorg (common ancestor, removed blocks, replacement blocks). It fails closed when no common ancestor exists inside the configured window.
8. The regtest integration test uses a disposable node, confirms a block and transaction, invalidates a disposable tip via the test fixture's bitcoin-cli wrapper, mines a replacement fork and asserts that the observer detects the reorg.
9. The Linux CI regtest container uses Docker host networking. RPC binds and allows only `127.0.0.1:18443`; no Docker port mapping is configured. P2P listening and outbound peers/DNS discovery are disabled. The RPC cookie is copied to a runner-owned mode-0600 temporary file for the host process; it is never printed.
10. No RPC observation is, by itself, a multi-node consensus proof or an ownership proof. A single RPC server can lie, be compromised, be stale or be reorged after the observation.

## Explicit limitations

- The reorg cursor is currently memory-only. Process restarts require a fresh bounded chain window. Persistent checkpointing, crash recovery and a deterministic rescan policy are not implemented.
- A reorg deeper than the configured retained window is not guessed through; the observer returns an error and requires an explicit resynchronization.
- The observer does not verify the node against independent peers/nodes, compact-filter quorum or a locally validated second chain source.
- The integration test's invalidateblock and generatetoaddress calls are test-fixture actions only. They are executed by a wrapper script in the disposable regtest environment, not by the Rust observer API.
- A valid block or transaction observation does not prove the maker controls spendable BTC, does not validate trade intent, does not prove USD payment and does not settle an exchange order.
- No PSBT construction/signing, custody, deposits, withdrawals, mainnet or customer-fund capability follows from this ADR.

## Consequences

BITE now has a tested local RPC boundary and can report the active-chain inclusion/confirmation state of a transaction in a specified block, as well as shallow reorg changes. The next work is to persist the chain cursor safely, define rescan/recovery after process restart, verify chain data independently, then add PSBT parsing and strict trade-intent comparison. Those later steps must remain regtest-only until separately reviewed.
