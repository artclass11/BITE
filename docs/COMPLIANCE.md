# Compliance and Mainnet Release Gate

**Status: preliminary engineering checklist, not legal advice. No real-value operation is approved by this document.**

BITE's first market, operating model, payment methods, target jurisdictions, and whether any entity performs a regulated activity have not been selected. These decisions must be made before enabling fiat rails, escrow, custody, deposits, withdrawals, or mainnet settlement.

## India-specific review gate

Before offering BITE services to users in India, obtain advice from qualified Indian counsel on the exact product and operating model. Review, at minimum:

- Whether BITE or any operator, affiliate, relay/indexer operator, escrow provider, or service partner falls within a regulated or reporting-entity activity.
- Whether a VDA service-provider registration or other registration, reporting, KYC/CDD, AML/CFT, recordkeeping, suspicious-transaction reporting, and customer due-diligence duties apply.
- Treatment of fiat payment rails, bank integrations, payment intermediaries, disputes, refunds, and third-party escrow/arbitration.
- Privacy and data-protection obligations, data minimisation, retention, cross-border processing, law-enforcement requests, and security incident reporting.
- Tax, consumer-protection, advertising, contract, sanctions, and payment-system requirements.
- Whether non-custodial software, a peer-to-peer marketplace, or relay/indexing services change any part of the analysis. Do not assume “non-custodial” automatically means “unregulated.”

FIU-IND publishes VDA-related AML/CFT guidance and registration materials. The applicable version and scope must be confirmed by counsel at the time of launch; regulatory material may change.

## U.S.-specific review gate for BTC/USD

USD is a quote currency, not a decision to serve U.S. customers or operate from the United States. Before accepting U.S. persons, using U.S. payment rails, or establishing a U.S. operating presence, obtain advice from qualified U.S. counsel on the exact protocol, matching, custody, escrow, payment and dispute functions.

Review, at minimum:

- Whether BITE, its operator, a matching/indexing service, relay, escrow provider, or payment partner is an exchanger or money transmitter under the Bank Secrecy Act and FinCEN's rules.
- Whether any limitation or exemption applies to the actual non-custodial or peer-to-peer arrangement; do not assume software-only, non-custodial, or decentralized architecture is exempt without a fact-specific analysis.
- FinCEN MSB registration, AML program, customer due diligence, recordkeeping and reporting obligations, if applicable.
- State-by-state money-transmission / virtual-currency licensing and any federal or state consumer-protection, sanctions, privacy, tax and payments requirements.
- CFTC anti-fraud and anti-manipulation jurisdiction over relevant virtual-currency spot-market conduct and whether any additional activity changes the analysis.
- USD payment-rail requirements, reversibility, ACH return risk, wire recalls, card chargebacks, fraud evidence, account name matching, holds, freezes and dispute processes.

FinCEN's 2019 guidance describes how its existing rules apply to certain convertible-virtual-currency business models. Its 2013 guidance addresses users, exchangers and administrators and explains that exchanger/administrator activities may be money transmission unless an applicable limitation or exemption applies. These are important starting points, not a legal conclusion about BITE. The CFTC also explains that it has general anti-fraud and anti-manipulation enforcement authority in virtual-currency cash markets and points users to MSB and state-level registration information.

Reference sources:

- [FinCEN FIN-2019-G001 — Application of FinCEN's Regulations to Certain Business Models Involving Convertible Virtual Currencies](https://www.fincen.gov/resources/statutes-regulations/guidance/application-fincens-regulations-certain-business-models)
- [FinCEN FIN-2013-G001 — Application of FinCEN's Regulations to Persons Administering, Exchanging, or Using Virtual Currencies](https://www.fincen.gov/resources/statutes-regulations/guidance/application-fincens-regulations-persons-administering)
- [CFTC — Be Smart: Check Registration & Backgrounds Before You Trade](https://www.cftc.gov/check)

This checklist is not legal advice and does not determine whether BITE or any participant is regulated. The assessment must be based on the deployed product and the current law in every target jurisdiction.

## Release blockers

Do not enable real users or real funds until all relevant items are evidenced and approved:

- [ ] Written jurisdiction-by-jurisdiction legal analysis for the actual BITE feature set.
- [ ] Confirmed legal entity, accountable operator, and responsibility split across any partners.
- [ ] Required registrations, approvals, reporting processes, and ongoing compliance controls completed.
- [ ] AML/CFT, customer due diligence, sanctions screening, suspicious activity escalation, and recordkeeping controls where applicable.
- [ ] Privacy notice, data inventory, retention schedule, and incident-response process.
- [ ] Payment-rail and escrow partner due diligence, dispute policy, and refund/recovery responsibilities.
- [ ] Independent review that the product UI accurately describes custody, counterparty, fiat, relay, and settlement risks.
- [ ] Independent security audit of the exact release commit and deployed configuration.
- [ ] Incident response, recovery, monitoring, key-management boundaries, and stop/pause procedures tested.
- [ ] Explicit maintainer approval with evidence linked from the release record.

A completed checklist is not a legal opinion or a certification of safety. Any material change to markets, custody, escrow, payment rails, or target countries reopens the review.

## Reference sources

- [FIU-IND downloads and VDA-related guidance](https://fiuindia.gov.in/files/downloads/downloads.html)
- [FIU-IND VDA service-provider registration circular (15 September 2025)](https://fiuindia.gov.in/pdfs/downloads/VDASP15092025.pdf)
- [Government of India / PIB release on VDA service-provider obligations](https://www.pib.gov.in/PressReleasePage.aspx?PRID=2308131&lang=1&reg=3)

Maintainers must verify current official materials and obtain professional advice before launch; this file is a change-control gate, not a substitute for that work.
