# BITE Security Policy

## Current security status

BITE is a research/prototype foundation. It is **not approved for mainnet trading or real funds**. There is no production exchange, customer custody, deposit service, withdrawal service, or audited escrow implementation in this repository.

Do not submit real seed phrases, private keys, wallet backups, authentication tokens, production RPC credentials, or customer information in issues, pull requests, logs, or test fixtures.

## Reporting a vulnerability

Please do not publish an exploitable vulnerability in a public issue.

Use GitHub's private vulnerability reporting / Security Advisories for this repository when available. Include:
- affected commit, branch, component, and configuration;
- reproducible steps and a minimal proof of concept;
- impact, preconditions, and whether funds or secrets could be affected;
- suggested mitigation, if known.

Do not test against other people's wallets, funds, infrastructure, or live markets. Do not exfiltrate data or keep access after confirming an issue.

## Security response expectations

The maintainers should acknowledge reports, assess severity and exploitability, coordinate a fix and disclosure date, publish an advisory where appropriate, and add a regression test. A fix is not considered complete until the root cause and affected trust boundary are documented.

## Release gates

A release must not handle mainnet value until all applicable gates are complete:
1. documented threat model and protocol specification;
2. independent review of transaction construction, scripts, signing, and state transitions;
3. unit, property, fuzz, integration, reorg, replay, and adversarial tests;
4. dependency and build provenance review;
5. operational controls, monitoring, incident response, and tested recovery;
6. legal/compliance review for supported jurisdictions and payment flows;
7. explicit maintainer approval of the release gate.

A successful CI run is not a security audit.