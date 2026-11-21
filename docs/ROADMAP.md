# BITE Roadmap

This is a gate-based roadmap, not a promised delivery schedule. A stage is complete only when its acceptance criteria and review evidence exist in the repository.

## Stage 0 — Project foundation

- [x] Create security-first project README.
- [x] Document initial architecture and trust boundaries.
- [x] Document initial threat model.
- [x] Add security and contribution policies.
- [x] Record the first product decision in an ADR (BTC/USD).
- [ ] Confirm supported jurisdictions, payment rails, and user flows.

**Exit gate:** product definition and core threat model reviewed.

## Stage 1 — Protocol specification

- [x] Add an initial BTC/USD order-payload validator and integer quote arithmetic prototype (not yet a signed Nostr protocol).
- [ ] Define canonical signed order schema and versioning.
- [x] Add a bounded in-process replay guard for event IDs and maker-scoped order IDs.
- [x] Add a versioned, bounded SQLite local replay store with atomic uniqueness checks and restart persistence.
- [ ] Define durable cross-client order identity, nonce, expiry, cancellation, partial-fill, and replay rules.
- [ ] Define deterministic trade state machine and transition authorization.
- [ ] Define fee, dust, amount, price-tick, partial-fill, rounding, and change-output policy.
- [x] Confirm USD as the quote currency and document the off-chain trust boundary.
- [ ] Select USD payment rails after jurisdiction-specific legal, fraud and reversibility review.
- [ ] Compare direct payment, 2-of-2 timeout, and 2-of-3 escrow models.
- [ ] Define node verification and confirmation/reorg policy.
- [ ] Write test vectors and protocol invariants.

**Exit gate:** independent protocol review before transaction code is allowed.

## Stage 2 — Offline / regtest prototype

- [x] Rust workspace for BTC/USD domain and protocol types.
- [x] Deterministic BTC/USD order validation with bounded integer parsing.
- [x] Verify Nostr event ID/signature and bind order maker/timestamp (BITE event kind/tags and global replay rules remain pending).
- [x] Persist local replay keys across restarts with SQLite (cross-device/global replay remains pending).
- [x] Add a local Bitcoin Core regtest sandbox and CI observer integration; the test forces `regtest`, disables P2P listening/outbound peers, and restricts RPC to host loopback on the Linux CI runner.
- [x] Add a read-only Rust Bitcoin Core RPC observer with cookie-file authentication, loopback-only endpoint validation and mandatory `chain=regtest` checks.
- [x] Verify block hash/height/body/ancestry, transaction inclusion and active-chain confirmations against the current tip.
- [x] Detect shallow regtest reorgs and report the common ancestor, disconnected blocks and replacement blocks; fail closed when the fork exceeds the retained window.
- [ ] Add persistent chain cursor/checkpoint storage, crash-safe rescan/recovery and independent multi-node verification.
- [ ] PSBT construction and strict validation.
- [ ] Simulated two-party signing with disposable test keys.
- [ ] Property and fuzz tests.
- [ ] No mainnet configuration in the default build.

**Exit gate:** tests prove no single application API can fabricate settlement or move user funds. The current observer verifies one regtest node's active-chain data and shallow reorgs; PSBT intent validation and controlled signing/settlement safety are not yet implemented, so the exit gate remains open.

**Stage 2 progress note:** durable local replay, the isolated Bitcoin Core regtest environment, and a read-only Rust observer with real regtest block/transaction/reorg tests are implemented and pass CI. Stage 2 is not complete: PSBT construction and strict intent validation, simulated two-party signing, persistent chain cursor/recovery, and property/fuzz coverage still block its exit gate.

## Stage 3 — Wallet and decentralized discovery

- [ ] Client-side wallet integration.
- [ ] Human-readable transaction review.
- [ ] Multi-relay signed order discovery.
- [ ] Relay censorship / outage / replay handling.
- [ ] Privacy and metadata review.
- [ ] Accessibility and phishing-resistant origin design.

**Exit gate:** independent clients can verify signed orders and transaction intent without trusting the indexer.

## Stage 4 — Adversarial test phase

- [ ] Reorg and conflicting-spend simulation.
- [ ] PSBT substitution and fee-manipulation suite.
- [ ] Relay flooding and malformed-message suite.
- [ ] RPC boundary and infrastructure penetration testing.
- [ ] Dependency, build, and release-provenance audit (Cargo.lock is committed and CI uses `--locked`; independent dependency review remains pending).
- [ ] External specialist protocol and cryptography review.
- [ ] Remediate and close critical/high findings.

**Exit gate:** no unresolved critical/high security findings; public test environment only.

## Stage 5 — Public test deployment

- [ ] Testnet deployment with explicit test-only branding.
- [ ] Public bug reporting and coordinated disclosure process.
- [ ] Monitoring, rate limiting, backups, and restore drill.
- [ ] Incident response exercise.
- [ ] No real funds, no mainnet settlement, no promise of production readiness.

**Exit gate:** documented operational readiness and audit findings closed.

## Stage 6 — Mainnet decision (not automatic)

- [ ] Jurisdiction-specific legal/compliance analysis.
- [ ] Independent audit of the exact release commit and deployed configuration.
- [ ] Finalized settlement and recovery model.
- [ ] Security incident response and key-compromise playbooks.
- [ ] Operational controls and independent monitoring.
- [ ] Formal release approval with a rollback / pause plan.

Mainnet remains disabled until every applicable gate is approved. Passing this roadmap does not certify that BITE is “Binance-level” or risk-free.