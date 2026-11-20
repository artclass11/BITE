# BITE

**BITE is a Bitcoin-only, peer-to-peer exchange project.**

The first selected market is **BTC/USD**: Bitcoin is the only supported crypto asset; USD is the quote/reference currency. BITE is designed around user-controlled signing and decentralized order discovery.

> **Status: Research / prototype foundation. Not production-ready.**
>
> This repository does not currently implement a live exchange, custody, deposits, withdrawals, escrow, USD payment processing, or real-money trading. Do not send bitcoin to an address or service claiming to be BITE.

## BTC/USD product boundary

- BTC is the only supported crypto asset. No altcoins, wrapped tokens or synthetic BTC.
- USD is the quote/reference unit. BUY and SELL order concepts are in scope.
- BTC amounts use integer satoshis. Limit prices use integer micro-USD per BTC; monetary fields must never use floating-point values.
- Selecting BTC/USD does not select ACH, wire, card, cash, payment apps, escrow, countries or customer eligibility.
- USD transfers occur outside Bitcoin consensus. Bitcoin Script cannot prove that an off-chain USD payment occurred. BITE must not describe BTC/USD fiat settlement as fully trustless.
- The prototype must not custody customer USD or BTC.

## Non-negotiable security principles

1. **No platform custody by default.** The application server must not hold user seed phrases, private keys, or pooled customer bitcoin.
2. **User-controlled signing.** Transactions are constructed as PSBTs and signed in the user's wallet or hardware signer. The server never asks for a seed phrase.
3. **No hot-wallet exchange balance.** There is no internal customer balance ledger that purports to represent redeemable bitcoin.
4. **Bitcoin consensus is authoritative.** A trade is not settled because an API, relay, webhook, or browser says so. Settlement requires independent chain verification and a clearly defined confirmation policy.
5. **Decentralized discovery, verifiable messages.** Order messages are signed by their authors and may be distributed through multiple independent relays. Relays are transport, not sources of truth.
6. **Fail closed.** Stale chain data, conflicting transactions, reorgs, unavailable signers, invalid signatures, and uncertain trade states must stop progression rather than guess.
7. **No mainnet experimentation.** Development and CI use regtest/test fixtures. Mainnet support is a separate release gate.
8. **Security review before value.** No production release before independent protocol review, threat-model review, adversarial testing, and a documented incident-response plan.

## Proposed architecture (subject to ADRs)

- **Client:** user-controlled wallet integration; local transaction review and signing; no seed/private-key collection.
- **Order discovery:** signed, schema-versioned events distributed across multiple Nostr-compatible relays. Clients verify event signatures and expiry locally.
- **Bitcoin integration:** isolated Bitcoin Core node(s), read-only public-chain observation for the application, explicit confirmation/reorg tracking, and PSBT-based transaction coordination.
- **Backend:** replaceable discovery/indexing services. Backend state is a cache; it is never the authority for ownership or settlement.
- **Settlement:** a separately reviewed Bitcoin-native mechanism. No escrow contract is assumed safe merely because it uses multisig or Taproot.
- **Operations:** reproducible builds, dependency pinning, signed releases, least-privilege infrastructure, monitoring, backups, and a tested recovery process.

## Executable prototype

Dependency-free offline prototypes are in [prototype/](prototype/):
- trade lifecycle transitions, PSBT-hash binding, confirmation thresholds, reorgs, conflicting spends, disputes, expiry, cancellation and refund reorgs;
- BTC/USD order payload validation, integer monetary units, exact rational quote arithmetic and BTC/USD price comparison.

The order validator checks payload shape only; it does not verify the maker's cryptographic signature.

Run the tests with Node.js 22 or later:

```sh
npm test
```

The JavaScript prototypes remain useful for fast state-machine and market-domain tests. The Rust workspace adds `crates/bite-protocol`, which:

- strictly parses the outer Nostr event and BTC/USD order payload (unknown/duplicate fields fail closed);
- verifies the Nostr event ID and Schnorr signature with the pinned `nostr = 0.45.7` library;
- binds the order's `makerPubkey` and `createdAt` to the verified event author and timestamp;
- enforces size limits, bounded integer parsing, BTC supply limits, order expiry and exact integer-rational quote arithmetic;
- provides a bounded in-process replay guard and a SQLite-backed durable replay store for duplicate event IDs and `(makerPubkey, orderId)` pairs, with expiry pruning, atomic insertion, and fail-closed capacity handling.

Run both test suites with Node.js 22 and a stable Rust toolchain:

```sh
npm test
cargo fmt --all -- --check
cargo test --locked --workspace
```

This is still a protocol-core milestone, not a deployed exchange. The BITE order event kind/tags, cross-device/global replay rules, cancellation, partial fills, relay network, Bitcoin Core observer, PSBT parsing, wallet signing, and settlement remain unimplemented. The in-memory replay guard is process-local. The SQLite replay store survives restarts and coordinates processes that share the same local database file, but it is not a distributed database and does not synchronize separate devices or clients. Restore from an old database backup can roll local replay history backward. The Nostr verifier checks event authenticity, not BTC ownership or USD payment. GitHub Actions runs the Node.js and Rust checks with the committed `Cargo.lock` on pushes and pull requests.

## Development stages

1. **Specification:** define market semantics, threat model, trade state machine, key ownership, and settlement assumptions.
2. **Offline prototype:** signed order messages, schema validation, deterministic state machine, and regtest-only transaction simulations.
3. **Wallet integration:** PSBT creation/review/signing with no server-side private keys.
4. **Adversarial testing:** fuzzing, property tests, relay spam/replay tests, reorg tests, transaction replacement tests, and independent code review.
5. **Limited public test:** testnet/regtest only, public bug bounty or coordinated disclosure process, and no real funds.
6. **Mainnet decision:** only after legal, security, operational, and independent audit gates are passed.

The BTC/USD order payload contract and current Rust verifier/replay-store boundary are documented in [docs/protocol/BTC_USD_ORDER_V1.md](docs/protocol/BTC_USD_ORDER_V1.md) and [ADR 0002](docs/adr/0002-durable-replay-store.md). The BITE-specific event kind, tags and cross-device replay protocol remain pending.

## Security status

See [SECURITY.md](SECURITY.md), [docs/THREAT_MODEL.md](docs/THREAT_MODEL.md), [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md), [docs/DECISIONS.md](docs/DECISIONS.md), [docs/adr/0001-btc-usd-market.md](docs/adr/0001-btc-usd-market.md), [docs/COMPLIANCE.md](docs/COMPLIANCE.md), [docs/ROADMAP.md](docs/ROADMAP.md), and [docs/IMPLEMENTATION_STATUS.md](docs/IMPLEMENTATION_STATUS.md).

## Technical references

- [Bitcoin Improvement Proposals](https://github.com/bitcoin/bips)
- [BIP 340 — Schnorr signatures](https://github.com/bitcoin/bips/blob/master/bip-0340.mediawiki)
- [BIP 341 — Taproot](https://github.com/bitcoin/bips/blob/master/bip-0341.mediawiki)
- [BIP 174 — PSBT](https://github.com/bitcoin/bips/blob/master/bip-0174.mediawiki)
- [Bitcoin Core RPC security guidance](https://github.com/bitcoin/bitcoin/blob/master/doc/JSON-RPC-interface.md)
- [Nostr NIP-01](https://github.com/nostr-protocol/nips/blob/master/01.md)
- [Rust Bitcoin library](https://docs.rs/bitcoin/latest/bitcoin/)

## Contributing

Read [CONTRIBUTING.md](CONTRIBUTING.md). Security-sensitive changes must include tests, threat-model updates, and a review note. Do not submit secrets, seed phrases, private keys, wallet files, or real customer data.

---

BITE is an engineering project, not investment advice, a bank, a broker, or a promise of returns.