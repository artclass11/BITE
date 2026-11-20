# BITE Threat Model (Initial)

**Status:** incomplete working document. It must be reviewed and expanded before any mainnet or real-value release.

## 1. Scope and security objectives

Protect:
- user bitcoin and the ability to spend it only under user-approved conditions;
- private keys, seed phrases, wallet metadata, and sensitive payment details;
- integrity of order terms, PSBTs, transaction outputs, and trade state;
- correctness of chain observation, confirmation counts, and reorganization handling;
- availability and integrity of signed orders and public protocol data.

Security objectives:
1. The service cannot unilaterally move a user's bitcoin.
2. A malicious relay or indexer cannot forge an order or settlement.
3. Users can independently inspect and verify transaction terms.
4. The system never silently changes destination, amount, fee, network, script, or expiry.
5. Uncertainty causes a safe pause, not a false success.
6. No single backend component should be the source of truth for ownership or settlement.

## 2. Assets and trust boundaries

| Asset | Primary protection |
|---|---|
| User signing keys / seed phrase | Never collected or stored by BITE |
| Unsigned and signed PSBTs | Strict policy validation; user wallet signs |
| Order integrity | Author signatures and schema validation |
| Settlement status | Self-operated Bitcoin Core verification and reorg tracking |
| Payment evidence | Private, minimized, and never treated as cryptographic proof |
| Relay/indexer data | Untrusted input; verify signatures and bounds |
| Build and release artifacts | Pinned dependencies, reproducible build goals, signed provenance |

Trust boundaries: browser/client, wallet or hardware signer, relay network, BITE API/indexer, Bitcoin Core RPC, external payment rails, and operations/deployment infrastructure.

## 3. Adversaries

- Malicious or compromised exchange operator.
- Malicious counterparty creating deceptive orders or submitting altered PSBTs.
- Malicious relay replaying, delaying, censoring, or dropping events.
- Compromised indexer/API returning stale or fabricated status.
- Phishing site or malicious browser extension imitating BITE.
- Network attacker intercepting or manipulating traffic.
- Dependency, build pipeline, or release-account compromise.
- Sybil/spam attacker creating many orders or relay identities.
- Chain-level adversary exploiting reorgs, fee pressure, transaction replacement, or mempool differences.
- Insider with excessive infrastructure or repository permissions.

## 4. Key threats and required controls

### Key theft or unauthorized signing
**Threat:** seed phrase or private key is collected by the site, leaked in logs, or accessed by a compromised server.

**Controls:** no key import or seed phrase form; wallet/hardware signing only; secret scanning; no secrets in telemetry; strict content security policy; dependency minimization; independent review of wallet integration.

### PSBT substitution
**Threat:** a malicious API, relay, browser, or counterparty changes outputs, amounts, fees, change, inputs, locktime, sequence, or script conditions between review and signing.

**Controls:** bind the PSBT to a signed trade intent; parse and validate every field locally; display human-readable outputs; reject unknown or unexpected fields; compare before and after signing; use wallet-level transaction review; include adversarial test vectors.

### Fake or replayed orders
**Threat:** forged signatures, expired offers, duplicate fills, altered terms, or stale relay events.

**Controls:** verify event signatures and canonical encoding; include a unique order ID and nonce; define expiry and replay rules; enforce deterministic idempotency; validate limits locally; treat relay timestamps as untrusted hints.

### False settlement / reorganization
**Threat:** a service marks a trade settled from a webhook, a single explorer, a mempool observation, or a block that is later reorganized out.

**Controls:** independently query a self-operated node; track txid, output, block hash, height, and confirmation depth; distinguish broadcast / mempool / confirming / settled; handle conflicts and reorgs; never equate API success with settlement.

### RPC compromise
**Threat:** public or weakly authenticated Bitcoin Core RPC allows control of node functions, private data exposure, or wallet spending.

**Controls:** separate host/container and OS user; bind RPC to a private interface; firewall and allowlist; authentication; least privilege; no browser access; dedicated credentials; prefer a node configuration without a loaded wallet for the observer.

### Fiat-payment fraud and dispute
**Threat:** one party lies about an external bank/cash payment, reverses a payment, submits forged evidence, or refuses to cooperate.

**Controls:** clearly explain that fiat settlement is off-chain and not enforced by Bitcoin; do not claim full trustlessness; define dispute rules, evidence retention, mediator powers, appeal and timeout behavior before supporting fiat rails. This risk cannot be eliminated by an on-chain transaction alone.

### Denial of service and relay abuse
**Threat:** spam floods relays/indexers or blocks legitimate order access.

**Controls:** rate limits, bounded payloads, quotas, per-key limits, backpressure, multiple relays, abuse monitoring, and graceful degradation. Avoid making a single relay mandatory.

### Supply-chain / maintainer compromise
**Threat:** malicious dependency, compromised maintainer account, poisoned release, or workflow token abuse.

**Controls:** minimal dependencies; pin versions; review transitive dependencies; least-privilege CI permissions; protect release credentials; signed tags/artifacts; build provenance; independent release approval; secret scanning.

### Privacy leakage
**Threat:** public order data links a person, wallet, IP address, or bank identity.

**Controls:** publish only minimum order data; no payment details in public events; separate identity from order keys where appropriate; avoid server-side IP logs where feasible; document retention; threat-model relay metadata and network-layer linkage.

## 5. Explicit non-goals for the prototype

- No production custody, pooled funds, deposits, withdrawals, or user balance claims.
- No live BTC/fiat settlement.
- No custom cryptographic primitive.
- No unaudited escrow or arbitration scheme.
- No claim of censorship resistance merely because several relays are configured.
- No guarantee against all chain reorganizations, network-level censorship, or off-chain payment fraud.

## 6. Test plan required before value

- Unit and property tests for event parsing, signature verification, amount arithmetic, fee calculations, and state transitions.
- Fuzz tests for malformed events, PSBTs, scripts, and RPC responses.
- Replay, duplicate, expiry, race, and out-of-order message tests.
- Reorg, conflicting transaction, mempool eviction, and replacement tests on regtest.
- Adversarial PSBT mutation suite covering every input and output field.
- RPC isolation and authentication checks.
- Dependency, secret, static-analysis, and container-image scans.
- Independent protocol and code audit with findings tracked to closure.

## 7. Residual risk

No design eliminates counterparty risk, external fiat-payment risk, malware on a user's own device, coercion, network-level correlation, or all consensus/reorg risk. BITE must disclose residual risks plainly and must not imply that non-custodial means risk-free.