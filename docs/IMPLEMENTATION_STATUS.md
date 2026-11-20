# BITE Implementation Status

**Snapshot date:** 2026-11-20  
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
- Rust unit/integration tests for valid signed orders, tampering, maker mismatch, timestamp mismatch, duplicate/unknown fields, invalid amounts, expiry, oversized input and policy validation.
- Exact rational quote-notional arithmetic; the prototype performs no floating-point monetary calculations and deliberately does not round settlement amounts.
- BTC/USD price comparison using integer arithmetic.
- Trade-state machine prototype with immutable aggregate transitions, state invariants, PSBT-hash binding, signature-policy milestones, confirmation thresholds, chain conflict handling, reorgs, refund-intent/transaction binding, refund finality and refund reorg handling.
- Automated tests for BTC/USD order validation and trade-state transitions.
- GitHub Actions workflow configured to run the Node.js test suite on pushes and pull requests; latest verified workflow run at the time of this snapshot reported 27 tests passed.
- CODEOWNERS file added; main-branch protection is not enabled yet and is tracked in GitHub issue #5.
- Initial architecture, threat model, security policy, contribution rules, compliance release checklist, decision log and staged roadmap.

## Not implemented

- Production frontend, backend API, accounts, live order book or relay service.
- Approved BITE-specific Nostr event kind/tag schema, global replay and order-uniqueness protection, cancellation/partial-fill semantics, anti-spam controls, and multi-relay implementation.
- Bitcoin Core integration or independently verified chain observer.
- PSBT parser, transaction construction, fee policy, wallet integration or hardware-signer flow.
- USD payment integrations, fiat escrow, arbitration, bank verification or refunds.
- Mainnet/testnet deployment, monitoring, backups, incident-response drills or independent security audits.
- Jurisdiction-specific legal/compliance approval.

## Current blockers

1. **USD rails and jurisdictions:** no ACH, wire, card, cash, payment-app or other USD rail is selected. Countries and customer eligibility require counsel review.
2. **Settlement model:** direct counterparty settlement, 2-of-2 timeout and 2-of-3 escrow have different recovery and trust assumptions. None is approved.
3. **Trust-boundary implementation:** the order validator checks payload structure only and does not authenticate a maker. State-machine events are trusted inputs. Neither currently verifies a PSBT, Bitcoin transaction, block, node response or USD payment.

## Next engineering sequence

1. Implement and review the signed BTC/USD Nostr event envelope, including event identity, nonce/replay protection, expiry, cancellation, partial fill and signature test vectors, based on docs/protocol/BTC_USD_ORDER_V1.md.
2. Select supported jurisdictions and a USD payment model after qualified legal review.
3. Compare direct settlement, cooperative Bitcoin contracts and escrow with a written dispute/recovery policy.
4. Implement multi-relay order discovery with independent relay failure, censorship and spam handling; the current Rust crate only verifies supplied signed events.
5. Implement an isolated Bitcoin Core observer with reorg and conflict tracking.
6. Implement PSBT parsing and strict comparison against agreed trade intent, followed by user-controlled wallet signing.
7. Run regtest adversarial simulations and independent review before any public test deployment.

## Release restriction

Mainnet and customer funds remain disabled. Prototype tests are not a security audit, and BITE must not be described as Binance-level, production-grade or risk-free.