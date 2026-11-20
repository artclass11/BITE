# BITE Roadmap

This is a gate-based roadmap, not a promised delivery schedule. A stage is complete only when its acceptance criteria and review evidence exist in the repository.

## Stage 0 — Project foundation

- [x] Create security-first project README.
- [x] Document initial architecture and trust boundaries.
- [x] Document initial threat model.
- [x] Add security and contribution policies.
- [ ] Record product and protocol decisions in ADRs.
- [ ] Confirm intended market, quote asset, jurisdictions, and user flows.

**Exit gate:** product definition and core threat model reviewed.

## Stage 1 — Protocol specification

- [ ] Define canonical signed order schema and versioning.
- [ ] Define order identity, nonce, expiry, cancellation, partial-fill, and replay rules.
- [ ] Define deterministic trade state machine and transition authorization.
- [ ] Define fee, dust, amount, rounding, and change-output policy.
- [ ] Decide whether fiat rails are in scope and disclose their trust assumptions.
- [ ] Compare direct payment, 2-of-2 timeout, and 2-of-3 escrow models.
- [ ] Define node verification and confirmation/reorg policy.
- [ ] Write test vectors and protocol invariants.

**Exit gate:** independent protocol review before transaction code is allowed.

## Stage 2 — Offline / regtest prototype

- [ ] Rust workspace for domain and protocol types.
- [ ] Deterministic order validation and state machine.
- [ ] Signed order message verification.
- [ ] Local regtest Bitcoin Core integration.
- [ ] PSBT construction and strict validation.
- [ ] Simulated two-party signing with disposable test keys.
- [ ] Property and fuzz tests.
- [ ] No mainnet configuration in the default build.

**Exit gate:** tests prove no single application API can fabricate settlement or move user funds.

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
- [ ] Dependency, build, and release-provenance audit.
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