# BITE Architecture (Initial Proposal)

**Status:** proposed architecture, not an audited protocol. Decisions marked open must be resolved before real-value functionality.

## 1. Product definition

BITE aims to be a Bitcoin-only peer-to-peer exchange. Bitcoin is the only supported crypto asset and the only on-chain settlement system.

A market still needs a counter-asset or quote unit. If a future market is BTC/fiat, the fiat payment happens outside Bitcoin consensus. Neither a relay nor a Bitcoin script can prove that a bank transfer or cash payment occurred. Therefore, BTC/fiat trades are not fully trustless; any escrow, mediator, bank evidence, or reputation dependency must be visible to users.

**Open product decision:** choose the first market and quote/payment rail before implementing settlement. Do not quietly infer that “Bitcoin-only” means BTC is the only asset, the only quote unit, or that fiat is supported.

## 2. Security boundaries

### Client / wallet boundary
- The user controls wallet keys and reviews every transaction.
- Signing happens in a compatible wallet or hardware signer.
- The application must never request or persist seed phrases, private keys, wallet exports, or signing-device secrets.
- The client displays the network, inputs, outputs, amounts, fee rate, change, locktime, sequence, and contract conditions before signing.

### Order-discovery boundary
- Orders are signed by their authors.
- Relays distribute events; they do not create, own, or validate orders on behalf of users.
- Clients validate signatures, schema version, timestamps, expiry, bounds, and replay identifiers.
- Multiple relays are supported; relay failure must not imply an order was cancelled or settled.
- The protocol must define event kinds, canonical serialization, versioning, idempotency, expiry, rate limits, and deletion semantics before implementation.

Nostr NIP-01 is a candidate transport baseline, not a complete BITE trading protocol.

### Bitcoin-node boundary
- Bitcoin Core runs as a separate, isolated service.
- RPC is not publicly exposed. Network allowlists, authentication, dedicated OS identity, restricted filesystem permissions, and separate credentials are required.
- The public-facing application does not receive unrestricted wallet RPC access.
- Chain observations include block hash, height, txid, vout, amount, scriptPubKey, confirmation depth, and reorg status.
- A second independent observation path may be used for alerting, but an explorer/API is never the sole settlement authority.

Bitcoin Core's RPC documentation warns that RPC access can control funds, affect consensus verification, and expose private data. Treat RPC as privileged control-plane access.

### Indexer / API boundary
- A backend may cache and index signed public orders and chain data.
- Cache state is reconstructible and is not authoritative for ownership, balances, or settlement.
- The API must not invent user balances or credit accounts based on unconfirmed deposits.
- No custodial account ledger is part of the initial design.

## 3. Proposed components

1. **BITE Client:** order creation, order verification, wallet connection, transaction review, local signing hand-off, status and risk disclosure.
2. **Relay Adapter:** publish and subscribe to signed order events across independent Nostr-compatible relays.
3. **Order Indexer:** rebuildable cache for search and discovery; validates signatures and schema before indexing.
4. **Bitcoin Observer:** reads blocks and transactions from a self-operated Bitcoin Core node; tracks confirmations, conflicts, mempool changes, and reorgs.
5. **Trade Coordinator:** deterministic state machine. It coordinates messages and PSBTs but cannot spend user funds.
6. **Audit/Event Log:** append-only operational events with no secrets or private wallet material.
7. **Operations Plane:** monitoring, backups, release signing, deployment controls, incident response.

Components should remain replaceable. No single backend service should be required for two users to verify each other's signatures or inspect a Bitcoin transaction.

## 4. Transaction coordination

PSBT is the proposed interchange format for unsigned and partially signed Bitcoin transactions. Prefer standard PSBT formats and wallet interoperability over custom signing payloads.

The coordinator should:
- construct unsigned transactions from validated public terms;
- expose every input, output, fee, change output, locktime, sequence, and script condition to both parties;
- validate the returned PSBT against the original proposal;
- reject unexpected inputs/outputs, amount changes, fee substitutions, sighash deviations, and unknown scripts;
- never accept a signature as proof that a transaction is confirmed;
- verify a broadcast transaction and track it through the chain state machine.

Taproot (BIP 341) and Schnorr signatures (BIP 340) are candidate building blocks, not a reason to invent a new contract. Any Taproot script tree, multisig policy, MuSig2 flow, timelock, or recovery path requires a written protocol specification, test vectors, and specialist review before it is used with value.

## 5. Trade state machine (initial vocabulary)

The final protocol must define valid transitions and authorization for each state. Suggested states:

- **DRAFT:** private local draft, not published.
- **OPEN:** signed and published order, not yet matched.
- **MATCH_PROPOSED:** a counterparty proposes a match; both parties must verify terms.
- **AGREED:** both parties acknowledge identical terms and expiry.
- **PSBT_PREPARED:** unsigned transaction matches the agreed terms.
- **PARTIALLY_SIGNED:** one or more expected signatures are present.
- **READY_TO_BROADCAST:** required signatures and script constraints validate.
- **BROADCAST:** transaction observed in the mempool.
- **CONFIRMING:** transaction is included in a block but has not met the defined finality policy.
- **SETTLED:** confirmation policy met and no known conflict or reorg invalidates the settlement.
- **EXPIRED, CANCELLED, REFUNDED, DISPUTED:** terminal or separately governed exception states.

A reorganization can move a previously confirming transaction back to an unconfirmed state. A previously displayed settlement must be corrected if the chain no longer supports it. The client must show this clearly.

No state is advanced solely by a webhook, relay message, client-side timer, or database write.

## 6. Settlement models — unresolved

Possible mechanisms have different trust assumptions and must not be conflated:

- **Direct payment:** simplest, but counterparties bear payment and counterparty risk.
- **2-of-2 cooperation with a timeout path:** no platform key, but a timeout does not resolve whether an off-chain fiat payment happened.
- **2-of-3 escrow:** may support dispute resolution, but introduces an arbitrator key and trust; it is not fully trustless.
- **Other Bitcoin-native contracts:** require a concrete threat model and review; do not market them as trustless without proving the exact properties.

No escrow model is selected by this architecture. Mainnet settlement remains out of scope until the decision is recorded in an ADR and independently reviewed.

## 7. Reliability and privacy

- Treat duplicate messages as normal; enforce idempotency.
- Use bounded queues, timeouts, backpressure, and abuse limits.
- Do not leak unnecessary IP-to-order associations.
- Do not publish private payment details in public relay events.
- Minimize retained personal data and define deletion/retention rules.
- Track chain reorganizations, conflicting spends, fee bumping, replacement-by-fee, and mempool eviction.
- Make the application usable when one relay, API, or indexer is offline.

## 8. References

- [Bitcoin BIPs](https://github.com/bitcoin/bips)
- [BIP 340 — Schnorr signatures](https://github.com/bitcoin/bips/blob/master/bip-0340.mediawiki)
- [BIP 341 — Taproot](https://github.com/bitcoin/bips/blob/master/bip-0341.mediawiki)
- [BIP 174 — PSBT](https://github.com/bitcoin/bips/blob/master/bip-0174.mediawiki)
- [BIP 370 — PSBT version 2](https://github.com/bitcoin/bips/blob/master/bip-0370.mediawiki)
- [Bitcoin Core JSON-RPC interface and security](https://github.com/bitcoin/bitcoin/blob/master/doc/JSON-RPC-interface.md)
- [Nostr NIP-01](https://github.com/nostr-protocol/nips/blob/master/01.md)
- [Rust Bitcoin](https://docs.rs/bitcoin/latest/bitcoin/)

These references are protocol starting points, not endorsements of this design.