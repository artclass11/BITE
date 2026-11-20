# BITE Decision Log

This file records decisions that materially affect security, decentralization, and product promises. New decisions should be added as dated Architecture Decision Records (ADRs), with alternatives and consequences.

## Open decisions

### D-001 — What does “Bitcoin-only exchange” mean for the first market?
**Status:** open.

Bitcoin is the only supported crypto asset. An exchange still needs a quote/counter-asset or a clear barter model. BTC/fiat introduces an off-chain payment leg that cannot be enforced by Bitcoin consensus. Before implementing trade settlement, select the exact first market and quote rail and document its trust assumptions.

### D-002 — Custody model
**Status:** proposed invariant.

BITE should be non-custodial by default. Application servers must not hold customer seed phrases, private keys, or pooled funds. Any future escrow contract must have its own ADR, threat model, independent review, and clear disclosure of arbitration or recovery powers.

### D-003 — Decentralized order discovery
**Status:** candidate, not approved.

Nostr-compatible signed events over multiple independent relays are a candidate. This provides signed, portable discovery messages but does not by itself solve spam, censorship, availability, or fiat settlement trust. The order event schema and protocol rules must be specified separately.

### D-004 — Bitcoin settlement model
**Status:** open.

Evaluate direct payment, 2-of-2 with timeout, and 2-of-3 escrow. Do not assume multisig equals trustless settlement. Specify who can spend, under what conditions, how refunds work, how disputes are decided, and what happens if a participant disappears.

### D-005 — Technology stack
**Status:** proposed for prototype.

Use Rust for protocol-critical transaction and state-machine logic, with standard maintained Bitcoin libraries. Choose the web/client stack after the protocol boundary is stable. Keep application services replaceable and avoid coupling custody or signing to the API.

## ADR template

For each decision, record:
- Title, date, status, owners/reviewers.
- Context and security constraints.
- Options considered.
- Decision and rationale.
- Security, privacy, decentralization, operational, and legal consequences.
- Rejected alternatives.
- Tests or evidence required before release.