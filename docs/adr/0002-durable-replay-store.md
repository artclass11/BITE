# ADR 0002 — Durable local replay store

- **Status:** Accepted for the Stage 2 prototype; not a production security approval
- **Date:** 2026-11-20
- **Scope:** Local BTC/USD order-ingest duplicate suppression

## Context

The first replay guard used bounded in-memory maps. It rejected a repeated Nostr event ID and a repeated `(makerPubkey, orderId)` pair while one process remained alive, but lost all state on restart. Multiple relay connections must also share one consistent admission path. Silently evicting a still-live order to make room would permit a replay to be admitted later.

This is a local client reliability problem. It is separate from global order identity, cancellation, partial-fill accounting, or consensus among independent devices.

## Decision

Implement `DurableReplayStore` with SQLite via pinned `rusqlite = 0.43.0` and its bundled SQLite feature.

1. Each local client uses one database file for every relay feed it ingests.
2. The database stores only the signed event ID, maker public key, order ID and order expiry. It stores no private keys, seeds, wallet material, bank/payment details or customer identity data.
3. The schema enforces a primary key on `event_id` and a unique constraint on `(maker_pubkey, order_id)`.
4. Each admission uses an IMMEDIATE SQLite transaction: prune rows whose expiry is at or before the caller's current time; reject duplicate event IDs; reject duplicate maker/order IDs; enforce capacity; insert; commit.
5. Live rows are never evicted to make room. At capacity, admission fails closed.
6. Capacity is bounded to 1–100,000 and fixed in database metadata at initial creation. Reopening with a different capacity fails rather than silently changing the limit.
7. The schema is versioned. Unknown schema versions and storage errors fail closed.
8. Database connections use WAL mode, FULL synchronous mode and a five-second busy timeout. The caller must retain the database and SQLite sidecar files as managed by SQLite; do not manually remove WAL/SHM files while the store is open.
9. Only a `VerifiedBtcUsdOrder` produced by the Rust signed-event verifier can enter the public store API.

## Explicit boundaries

- A shared SQLite file coordinates connections and processes that can access that same file. It does not synchronize different devices, independent clients or different database files.
- This is not global replay protection, a decentralized identity registry, a matching engine, or a payment/settlement mechanism.
- Restoring an old database backup can roll replay history backward. Production recovery must define how clients resynchronize and revalidate still-live orders after rollback.
- SQLite durability depends on the underlying storage and filesystem honoring their durability guarantees. Disk failure, file corruption, permission changes or storage exhaustion must stop order admission.
- The in-memory `ReplayGuard` remains available for bounded ephemeral use, but the durable store is the preferred local prototype path.
- The BITE-specific Nostr event kind and tag schema are deliberately not assigned by this ADR. Generic Nostr event signature verification does not itself define the BITE wire protocol.
- Nonces, cancellation, order replacement, partial fills, relay convergence and cross-device duplicate handling remain separate protocol decisions.

## Consequences

The prototype can reject locally observed replays after restart and can share duplicate state among local processes using the same database. It now has a persistent schema/configuration contract that requires migration discipline.

This decision does not authorize a public deployment or real-money use. Before production, the replay design needs independent review, tested backup/restore and corruption behavior, and an explicit cross-client protocol. No mainnet or customer-fund capability follows from this ADR.
