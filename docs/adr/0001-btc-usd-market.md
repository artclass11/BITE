# ADR 0001: Select BTC/USD as the first BITE market

- **Date:** 2026-11-20
- **Status:** Accepted for product scope; settlement and jurisdiction details remain open.
- **Decision owner:** BITE maintainers

## Context

The project requires a concrete first market before defining its order schema and pricing model. The product requirement is Bitcoin-only: no other crypto assets, wrapped assets, or synthetic BTC. The user selected USD as the quote currency.

## Decision

The first market is **BTC/USD**.

- Bitcoin (BTC) is the only supported crypto asset.
- USD is the quote and reference currency.
- Both BUY and SELL order sides are in scope.
- Order size is denominated in integer satoshis.
- Limit price is represented internally as integer micro-USD per BTC (1 USD = 1,000,000 micro-USD). No floating-point numbers are permitted in protocol monetary fields.
- The initial public order schema contains no bank account, card, payment-app, or other sensitive payment details.
- The market selection does not choose a USD payment rail, escrow mechanism, jurisdiction, or legal operating model.

## Consequences

1. Quote notional must be computed with integer/rational arithmetic. No JavaScript floating-point calculation may determine a signed order value.
2. Payment rounding, fees, minimum order size, price tick, amount precision, and partial-fill semantics remain protocol decisions and must be fixed in later ADRs.
3. USD is an off-chain settlement leg. Bitcoin consensus cannot prove that an ACH, wire, card, cash, or other off-chain USD payment occurred. Do not call BTC/USD settlement fully trustless.
4. BITE must not custody customer USD or BTC in this prototype.
5. Mainnet settlement remains disabled pending security, legal, compliance, operational, and independent review.

## Alternatives considered

- BTC/INR: not selected for the first market.
- BTC-only transfer tool without fiat price discovery: not selected for the first market.
- Multi-asset exchange: rejected because it conflicts with the Bitcoin-only scope.

## Open follow-on decisions

- Select supported countries and customer eligibility.
- Choose and review USD payment rails, including reversibility, fraud, chargebacks, evidence and disputes.
- Decide direct counterparty settlement versus escrow; document key control and recovery.
- Specify order-event signing, replay prevention, order expiry, partial fill and fee rounding.
- Obtain jurisdiction-specific legal advice before operating or facilitating trades.
