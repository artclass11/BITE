# Contributing to BITE

BITE handles security-critical protocol design. Correctness and clear threat boundaries matter more than feature count.

## Before opening a change

- Read SECURITY.md, docs/ARCHITECTURE.md, and docs/THREAT_MODEL.md.
- Open an issue or design proposal for protocol, cryptography, transaction, custody, or settlement changes.
- Keep changes small and independently reviewable.
- State assumptions, failure behavior, and what happens during a chain reorganization or network partition.

## Non-negotiable rules

- Never request, accept, log, or transmit a user's seed phrase or private key.
- Never add a server-side hot wallet or pooled customer balance as a shortcut.
- Never silently switch from regtest/testnet to mainnet.
- Never trust a relay, indexer, explorer, webhook, browser, or single API response as proof of settlement.
- Never report a trade as final while its settlement policy is unmet.
- Do not implement a custom Bitcoin script or signing protocol without an explicit design review and independent test vectors.
- Never put secrets, wallet files, real addresses tied to people, or customer data in the repository.
- No test may send real funds.

## Required evidence

For code changes, include tests for normal behavior, malformed input, adversarial input, failure/retry, and relevant state transitions. For protocol changes, update the threat model, architecture, state machine, and decision records. For dependencies, explain why the dependency is needed and review its maintenance, license, and security posture.

## Review standard

At least one independent reviewer should understand the change and its failure modes. Security-sensitive transaction and cryptographic changes need specialist review; ordinary code review is not a substitute.

## Development environment

The project must remain safe-by-default. Local development and automated tests should use regtest, deterministic fixtures, and throwaway keys. Mainnet mode must require a separate explicit configuration and must remain disabled until the release gates are approved.