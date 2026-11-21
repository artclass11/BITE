# BITE Implementation Status

**Snapshot date:** 2026-11-21  
**Readiness:** early prototype only; not suitable for mainnet or customer funds.

## Accepted product decision

- First market: BTC/USD.
- Bitcoin is the only supported crypto asset.
- USD is the quote/reference unit.
- BTC quantities are integer satoshis; prices are integer micro-USD per BTC.
- USD payment rails, supported jurisdictions, escrow and the settlement model remain unselected.

## Implemented in the repository

- Security-first README and explicit non-custodial design principles.
- ADR 0001 selecting BTC/USD and documenting off-chain USD settlement limitations.
- BTC/USD order payload v1 prototype contract documenting fields, integer units, exact quote arithmetic, time policy, proposed Nostr envelope requirements and unresolved semantics.
- BTC/USD order-payload validator with canonical integer strings, market and side restrictions, maker/order identifier format checks, expiry checks and unknown-field rejection.
- Rust workspace with a strict BTC/USD order type and a Nostr event verifier. It verifies event ID/signature, rejects unknown/duplicate outer and order fields, bounds input sizes, and binds maker key and timestamp to the signed event.
- Rust unit/integration tests for valid signed orders, tampering, maker mismatch, timestamp mismatch, duplicate/unknown fields, invalid amounts, expiry, oversized input, replay rejection, maker-scoped order uniqueness, expiry pruning and replay-guard capacity.
- Bounded in-process replay guard plus a SQLite-backed durable replay store. The durable store persists event IDs and maker-scoped order IDs across restarts, uses unique database constraints and atomic IMMEDIATE transactions, prunes only expired entries, fixes capacity in versioned metadata, and fails closed on duplicates, capacity and storage errors.
- Committed `Cargo.lock`; Rust CI runs `cargo test --locked --workspace`.
- Isolated Bitcoin Core 32.1 regtest Compose sandbox and Rust observer integration. RPC is forwarded only to host loopback (127.0.0.1:18443) for the test process; no P2P port is published, and the Docker network is internal.
- New `bite-bitcoin-observer` Rust crate using pinned `corepc-client` 0.19.0 async RPC with cookie-file authentication. It refuses non-regtest nodes and non-loopback endpoints, verifies chain-tip consistency, block hash/height/body/previous-block linkage, transaction inclusion and active-chain confirmations.
- In-memory `ChainTracker` reports initialization, unchanged tips, extensions and reorgs with common ancestor, removed blocks and added blocks; it fails closed if the reorg exceeds the configured 1–2,016 block window.
- End-to-end regtest integration test confirms a real block and transaction, then invalidates the disposable tip and mines a replacement fork to exercise reorg detection.
- Exact rational quote-notional arithmetic; the prototype performs no floating-point monetary calculations and deliberately does not round settlement amounts.
- BTC/USD price comparison using integer arithmetic.
- Trade-state machine prototype with immutable aggregate transitions, state invariants, PSBT-hash binding, signature-policy milestones, confirmation thresholds, chain conflict handling, reorgs, refund-intent/transaction binding, refund finality and refund reorg handling.
- Automated tests for BTC/USD order validation and trade-state transitions.
- GitHub Actions runs 27 JavaScript tests and 24 Rust tests (9 unit + 15 integration), plus rustfmt and Clippy with warnings denied. Verified all-green run: [44785971036](https://github.com/artclass11/BITE/actions/runs/44785971036).
- The same run passed the Bitcoin Core 32.1 regtest smoke test: `chain=regtest`, 101 blocks, no RPC/P2P ports published, followed by container/volume cleanup.
- CODEOWNERS file added; main-branch protection is not enabled yet and is tracked in GitHub issue #5.
- Initial architecture, threat model, security policy, contribution rules, compliance release checklist, decision log and staged roadmap.

## Not implemented

- Production frontend, backend API, accounts, live order book or relay service.
- Approved BITE-specific Nostr event kind/tag schema, cross-device/global replay protection, cancellation/partial-fill semantics, anti-spam controls, and multi-relay implementation. SQLite replay protection is local to one shared database file and is not network-wide.
- Persistent chain cursor/checkpoint storage, crash-safe rescan/recovery, independent multi-node chain verification and a production node trust policy. The current reorg cursor is in-memory and must be rebuilt from a fresh chain window after restart.
- PSBT parser, transaction construction, fee policy, wallet integration or hardware-signer flow.
- USD payment integrations, fiat escrow, arbitration, bank verification or refunds.
- Mainnet/testnet deployment, monitoring, backups, incident-response drills or independent security audits.
- Jurisdiction-specific legal/compliance approval.

## Current blockers

1. **USD rails and jurisdictions:** no ACH, wire, card, cash, payment-app or other USD rail is selected. Countries and customer eligibility require counsel review.
2. **Settlement model:** direct counterparty settlement, 2-of-2 timeout and 2-of-3 escrow have different recovery and trust assumptions. None is approved.
3. **Trust-boundary implementation:** signed Nostr events are authenticated by the Rust verifier, and a local SQLite replay store now persists duplicate keys. BITE-specific event kind/tags and cross-device/global replay rules remain pending. State-machine events are trusted inputs. Neither currently verifies a PSBT, Bitcoin transaction, block, node response or USD payment.

## Next engineering sequence

1. Finalize the signed BTC/USD Nostr event envelope and cross-client replay protocol: event kind/tags, nonce, expiry, cancellation, partial fill and published signature test vectors, based on docs/protocol/BTC_USD_ORDER_V1.md. Review the local SQLite store's backup/restore and corruption recovery behavior.
2. Select supported jurisdictions and a USD payment model after qualified legal review.
3. Compare direct settlement, cooperative Bitcoin contracts and escrow with a written dispute/recovery policy.
4. Implement multi-relay order discovery with independent relay failure, censorship and spam handling; the current Rust crate only verifies supplied signed events.
5. Add persistent chain cursor/checkpoint storage, crash-safe rescan/recovery and independent multi-node verification on top of the tested read-only observer.
6. Implement PSBT parsing and strict comparison against agreed trade intent, followed by user-controlled wallet signing.
7. Run regtest adversarial simulations and independent review before any public test deployment.

## Verified Stage 2 observer CI

The current observer milestone is verified by the complete CI run recorded on the draft PR. Its end-to-end regtest test executes a real tip invalidation and replacement fork. This remains a local single-node test, not an independent consensus proof.

## Release restriction

Mainnet and customer funds remain disabled. Prototype tests are not a security audit, and BITE must not be described as Binance-level, production-grade or risk-free.