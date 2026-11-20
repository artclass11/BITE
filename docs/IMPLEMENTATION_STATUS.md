# BITE Implementation Status

**Snapshot date:** 2026-11-20  
**Readiness:** early prototype only; not suitable for mainnet or customer funds.

## Implemented in the repository

- Bitcoin-only product boundary: BTC is the only supported crypto asset.
- Accepted first-market decision: BTC/USD, with integer satoshi size and integer micro-USD/BTC price representation. USD payment rails and supported jurisdictions remain undecided.
- Initial architecture and trust-boundary document.
- Threat model covering key theft, PSBT substitution, replay, false settlement, reorgs, RPC compromise, fiat fraud, relay abuse, supply-chain compromise, and privacy leakage.
- Security reporting policy, contribution rules, and compliance/mainnet release checklist.
- Decision log and ADR for BTC/USD.
- Dependency-free offline JavaScript trade state machine with immutable transitions, confirmation/reorg handling, aggregate consistency checks, and refund intent/transaction binding.
- BTC/USD order-domain primitives using canonical integer strings and exact rational quote arithmetic.
- GitHub Actions CI for Node.js 22. The latest completed code-test run executed **26 tests: 26 passed, 0 failed**.

## Not implemented

- Production frontend, backend API, accounts, live order book, or Nostr relay service.
- Canonical Nostr order-event signing/verification, authorization, replay protection across clients, anti-spam, or multi-relay discovery.
- Bitcoin Core node integration or independently verified chain observer.
- Production PSBT parser, transaction construction, fee/dust policy, wallet integration, or hardware-signer flow.
- Escrow, arbitration, timeout/refund contract, or real settlement.
- Mainnet/testnet deployment, production monitoring, backup/restore drills, incident-response exercises, or independent audits.
- Jurisdiction-specific legal analysis, required registrations, or an approved USD payment rail.

## Current blockers

1. **Settlement model:** direct payment, 2-of-2 timeout, and 2-of-3 escrow have different recovery and trust assumptions. None is approved.
2. **USD rails and jurisdictions:** BTC/USD is the first market, but the USD payment method, supported countries, and legal operating model are open.
3. **Trust-boundary implementation:** the prototype state machine accepts already-validated events. It does not authenticate event sources or independently verify Nostr signatures, PSBTs, transactions, blocks, or node responses.
4. **Operational assurance:** no production environment, recovery drill, specialist audit, or release approval exists.

## Next engineering sequence

1. Define a canonical signed BTC/USD order event, including nonce/identity, cancellation, expiry, partial fills, replay rules, price ticks, minimum size, and quote rounding.
2. Specify and compare direct settlement, 2-of-2 timeout, and 2-of-3 escrow with adversarial recovery scenarios.
3. Implement protocol-critical logic in a strongly typed Rust crate, with property and fuzz tests.
4. Implement Nostr signature verification and multi-relay discovery.
5. Implement an isolated Bitcoin Core observer with reorg, conflict, mempool, and replacement tracking.
6. Implement PSBT parsing and strict comparison against agreed trade intent, then integrate user-controlled signing.
7. Run regtest adversarial simulations and independent specialist review before any public test deployment.

## Release restriction

Mainnet and customer funds remain disabled. Passing CI validates only the tested prototype behavior; it is not a security audit. BITE must not be described as Binance-level, production-grade, risk-free, or ready for real trading.
