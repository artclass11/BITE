# BITE Implementation Status

**Snapshot date:** 2026-11-20  
**Readiness:** early prototype only; not suitable for mainnet or customer funds.

## Implemented in the repository

- Security-first README and explicit non-custodial design principles.
- Initial architecture and trust-boundary document.
- Initial threat model covering key theft, PSBT substitution, replay, false settlement, reorgs, RPC compromise, fiat fraud, relay abuse, supply-chain compromise, and privacy leakage.
- Security reporting policy and contribution rules.
- Open product/protocol decision log.
- Dependency-free offline JavaScript state-machine prototype.
- Eleven local tests for legal/illegal transitions, terms and PSBT hash mismatch, confirmation thresholds, chain conflicts, reorgs, expiry, cancellation, disputes, refund finality and refund reorgs.
- GitHub Actions workflow configured to run the prototype tests on pushes and pull requests.

## Not implemented

- Production frontend, backend API, accounts, order book or relay service.
- Nostr order-event schema, signature verification, anti-spam or replay-resistance implementation.
- Bitcoin Core node integration or independently verified chain observer.
- PSBT parser, transaction construction, fee policy, wallet integration or hardware-signer flow.
- Escrow, arbitration, timeout/refund contract or real settlement.
- Mainnet/testnet deployment, monitoring, backups, incident response drills or independent audits.
- Jurisdiction-specific legal/compliance assessment.

## Current blockers

1. **Market definition:** Bitcoin is the only crypto asset, but a market still needs a quote/counter-asset. A BTC/fiat market has an off-chain payment leg that Bitcoin consensus cannot verify.
2. **Settlement model:** direct payment, 2-of-2 timeout, and 2-of-3 escrow have different recovery and trust assumptions. None is approved.
3. **Trust boundary implementation:** the current state machine accepts already-validated events; it does not authenticate the event source or verify a signature, PSBT, transaction, block, or node response.

## Next engineering sequence

1. Resolve the first market, quote unit/payment rails, target jurisdictions, and settlement assumptions.
2. Write protocol ADRs and a canonical signed order schema with replay/expiry rules.
3. Port protocol-critical state and transaction validation into a memory-safe, strongly typed implementation and add property/fuzz tests.
4. Implement Nostr event signature validation and multi-relay order discovery.
5. Implement an isolated Bitcoin Core observer with reorg and conflict tracking.
6. Implement PSBT parsing and strict comparison against the agreed trade intent, then integrate user-controlled signing.
7. Run regtest adversarial simulations and independent review before any public test deployment.

## Release restriction

Mainnet and customer funds remain disabled. Test coverage in this prototype is not a security audit, and the project must not be described as Binance-level, production-grade, or risk-free.