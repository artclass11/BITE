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
