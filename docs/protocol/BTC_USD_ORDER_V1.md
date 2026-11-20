# BITE BTC/USD Order Payload v1 (Prototype Contract)

**Status:** prototype payload contract only. It is not yet a signed Nostr protocol, a matching engine, or a settlement specification.

The JavaScript structural validator is in `prototype/btc-usd-market.mjs`. The Rust verifier is in `crates/bite-protocol`; it parses a strict Nostr event, verifies the event ID and Schnorr signature, parses the order payload, and binds its maker key and creation timestamp to the event.

The Rust core is still a prototype. The BITE-specific event kind, tag layout, cancellation/partial-fill semantics and relay operation are not implemented or approved. The crate has both a bounded in-process replay guard and a SQLite-backed durable replay store. The SQLite store survives restarts and coordinates connections/processes that share one database file, but it is not shared across separate devices or independent clients. The `nostr` dependency is pinned to version `0.45.7` and `rusqlite` to `0.43.0`; neither dependency has been independently audited as part of BITE.

## Market and units

- Market identifier: `BTC-USD`.
- BTC is the only supported crypto asset.
- USD is the quote/reference unit; no USD payment method is implied.
- BTC size is an integer count of satoshis.
- Limit price is an integer count of micro-USD per BTC. One USD equals 1,000,000 micro-USD.
- Monetary values are canonical unsigned base-10 strings, not JSON numbers. They contain no sign, exponent, decimal point, whitespace, or leading zero (except the string `"0"`, which is rejected for positive values).
- All quote calculations use integer/rational arithmetic.

## Version 1 fields

| Field | Type | Rule |
|---|---|---|
| `version` | integer | Exactly `1` |
| `market` | string | Exactly `BTC-USD` |
| `orderId` | string | 32-byte lowercase hexadecimal client order identifier |
| `makerPubkey` | string | 32-byte lowercase hexadecimal maker public key |
| `side` | string | `BUY` or `SELL` |
| `priceUsdMicrosPerBtc` | string | Positive canonical integer micro-USD per BTC |
| `sizeSats` | string | Positive canonical integer satoshis; not greater than total Bitcoin supply in satoshis |
| `createdAt` | integer | Unix timestamp in seconds |
| `expiresAt` | integer | Unix timestamp in seconds, later than `createdAt` |

Unknown fields are rejected by the v1 validator. Payment instructions, personal details, bank identifiers, card data, account numbers, e-mail addresses, and private contact data must not be placed in public order payloads.

## Exact quote arithmetic

For a price (P) expressed as micro-USD per BTC and a size (S) expressed in satoshis:

`quote micro-USD = (P × S) / 100,000,000`

The implementation returns the numerator and denominator as decimal strings. It does not round. The amount sent over a USD payment rail, quote rounding, fee allocation, minimum order size, price tick, and partial-fill math must be specified in a later reviewed protocol decision before the market can settle trades.

## Implemented replay protection prototypes

The Rust crate exposes `ReplayGuard` for a single-process ingest loop:

- deduplicates an identical signed Nostr event by event ID;
- rejects a new event that republishes the same order ID from the same maker key;
- permits the same order ID under a different maker key (the namespace is maker + order ID);
- prunes entries only after their signed order expiry;
- has a bounded maximum capacity and rejects new orders when full rather than evicting still-live entries.

All relay connections handled by one process must feed the same in-memory guard through a serialized ingest path. It is lost on restart.

For restart persistence, `DurableReplayStore` uses SQLite. It stores event IDs, maker public keys, order IDs and signed expiries, with database uniqueness constraints on event ID and `(makerPubkey, orderId)`. Each admission uses one IMMEDIATE transaction to prune expired rows, check duplicates, enforce capacity and insert. It fails closed on storage errors, unsupported schema versions, changed capacity settings and out-of-range timestamps. `rusqlite` is pinned with bundled SQLite; the database uses WAL mode and FULL synchronous durability.

A durable store coordinates connections/processes only when they share the same local database file. It does not synchronize separate devices, different clients or independent database files. Restoring an old database backup can roll replay history backward; production recovery and cross-client order identity still need a reviewed protocol. Neither replay store defines nonce, cancellation or partial-fill semantics.

## Nostr event verification boundary (partially implemented)

The Rust verifier accepts a supplied Nostr event and verifies its event ID and Schnorr signature using the pinned `nostr` library. It strictly parses the outer event and the signed order content, and binds the payload maker and timestamp to the event author and timestamp. The integration tests currently construct valid events using a generic text-note kind only as a cryptographic fixture; this does **not** define the BITE event kind or tag format.

The following requirements are mandatory before this can be treated as a production BITE order protocol:

1. The Nostr event author's public key must equal the payload's `makerPubkey`.
2. Event ID and Schnorr signature must be verified locally by every client; a relay or indexer response is not proof of authorship.
3. The BITE event kind and tag layout remain unassigned pending a separate protocol ADR and independent review.
4. Event timestamp must be checked against `createdAt`; expiry and replay rules must be enforced by every client.
5. Duplicate JSON keys, oversized payloads, invalid UTF-8, non-canonical numeric values, unknown schema versions, and unsupported fields must fail closed.
6. Order IDs must be unique per maker within the defined retention window. Replays and duplicate fills must be idempotently rejected.
7. Clients must work with more than one independent relay and must treat relay content as untrusted input.
8. The exact serialized event content is signature-bound; clients must never parse an event, silently normalize it, and then verify a different serialization.

The event kind, tags, cancellation event, partial-fill semantics, nonce design, public/private metadata and retention policy are open protocol decisions. The Rust implementation verifies the Nostr event ID/signature, checks the maker key and creation time, and supports local durable duplicate suppression. It does not enforce global or cross-device order uniqueness, relay-wide cancellation, maker authorization beyond control of the Nostr signing key, or the BITE-specific event kind and tags. It does not prove that the maker owns or can spend the Bitcoin offered.

## Prototype time policy

The current validator defaults to a seven-day maximum order lifetime and allows thirty seconds of future clock skew. Those are prototype defaults, not yet approved market rules. Production policy must be explicit, bounded, tested across clock drift, and reviewed for relay retention and user cancellation.

## Security boundaries and non-goals

- The order validator is not a signature verifier.
- `makerPubkey` is not authenticated by the validator alone.
- A valid order is not proof that the maker owns or can spend the offered bitcoin.
- An order is not a signed transaction, a completed trade, or proof of USD payment.
- The order payload does not create escrow, custody, or a customer balance.
- USD settlement remains external to Bitcoin consensus.

## Acceptance gate

Do not use this payload as a production exchange protocol until the BITE event kind/tags, canonical/raw parsing, cryptographic verification, cross-client replay/cancellation/partial-fill rules, test vectors, privacy review, recovery behavior and independent protocol review have been completed.
