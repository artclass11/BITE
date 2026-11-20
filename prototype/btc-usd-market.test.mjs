import test from "node:test";
import assert from "node:assert/strict";
import {
  MARKET,
  MAX_BITCOIN_SUPPLY_SATS,
  validateBtcUsdOrder,
  calculateQuoteMicroUsdRational,
  compareBtcUsdPrices
} from "./btc-usd-market.mjs";

const NOW = 1_800_000_000;
const VALID_ORDER = Object.freeze({
  version: 1,
  market: MARKET,
  orderId: "a".repeat(64),
  makerPubkey: "b".repeat(64),
  side: "BUY",
  priceUsdMicrosPerBtc: "100000000000",
  sizeSats: "1000",
  createdAt: NOW - 10,
  expiresAt: NOW + 600
});

function validate(overrides = {}) {
  return validateBtcUsdOrder({ ...VALID_ORDER, ...overrides }, { nowSeconds: NOW });
}

test("valid BTC/USD order is normalized without floating point", () => {
  const order = validate();
  assert.equal(order.market, "BTC-USD");
  assert.equal(order.side, "BUY");
  assert.equal(order.priceUsdMicrosPerBtc, "100000000000");
  assert.equal(order.sizeSats, "1000");
  assert.ok(Object.isFrozen(order));
});

test("only BTC-USD market is accepted", () => {
  assert.throws(() => validate({ market: "ETH-USD" }), (error) => error.code === "UNSUPPORTED_MARKET");
});

test("only BUY and SELL sides are accepted", () => {
  assert.throws(() => validate({ side: "SHORT" }), (error) => error.code === "INVALID_SIDE");
});

test("monetary values must be canonical integer strings, never floats", () => {
  assert.throws(() => validate({ sizeSats: 1.25 }), (error) => error.code === "NON_CANONICAL_INTEGER");
  assert.throws(() => validate({ sizeSats: "01" }), (error) => error.code === "NON_CANONICAL_INTEGER");
  assert.throws(() => validate({ priceUsdMicrosPerBtc: "1.0" }), (error) => error.code === "NON_CANONICAL_INTEGER");
});

test("zero and out-of-range quantities are rejected", () => {
  assert.throws(() => validate({ sizeSats: "0" }), (error) => error.code === "OUT_OF_RANGE");
  assert.throws(() => validate({ sizeSats: (MAX_BITCOIN_SUPPLY_SATS + 1n).toString() }), (error) => error.code === "OUT_OF_RANGE");
});

test("oversized decimal strings are rejected before integer conversion", () => {
  assert.throws(() => validate({ sizeSats: "9".repeat(10000) }), (error) => error.code === "OUT_OF_RANGE");
  assert.throws(() => validate({ priceUsdMicrosPerBtc: "9".repeat(10000) }), (error) => error.code === "OUT_OF_RANGE");
});

test("order identity fields must have exact 32-byte lowercase hex encoding", () => {
  assert.throws(() => validate({ orderId: "xyz" }), (error) => error.code === "INVALID_ORDER_ID");
  assert.throws(() => validate({ makerPubkey: "C".repeat(64) }), (error) => error.code === "INVALID_PUBKEY");
});

test("expired, reversed and too-long orders are rejected", () => {
  assert.throws(() => validate({ expiresAt: NOW }), (error) => error.code === "ORDER_EXPIRED");
  assert.throws(() => validate({ expiresAt: NOW - 20 }), (error) => error.code === "INVALID_EXPIRY");
  assert.throws(() => validate({ expiresAt: VALID_ORDER.createdAt + 604_801 }), (error) => error.code === "EXPIRY_TOO_FAR");
});

test("future-dated orders beyond allowed clock skew are rejected", () => {
  assert.throws(() => validate({ createdAt: NOW + 31 }), (error) => error.code === "CREATED_IN_FUTURE");
});

test("unknown fields are rejected to avoid ambiguous signed payload semantics", () => {
  assert.throws(() => validate({ feePercent: 0.2 }), (error) => error.code === "UNKNOWN_FIELD");
});

test("BTC/USD quote notional is returned exactly as a rational micro-USD amount", () => {
  const result = calculateQuoteMicroUsdRational({
    priceUsdMicrosPerBtc: "100000000000",
    sizeSats: "1000"
  });
  assert.deepEqual(result, {
    numerator: "100000000000000",
    denominator: "100000000",
    unit: "microUSD"
  });
  assert.equal(BigInt(result.numerator) / BigInt(result.denominator), 1_000_000n);
});

test("BTC/USD prices compare using integer arithmetic", () => {
  assert.equal(compareBtcUsdPrices("100000000000", "100000000001"), -1);
  assert.equal(compareBtcUsdPrices("100000000000", "100000000000"), 0);
  assert.equal(compareBtcUsdPrices("100000000001", "100000000000"), 1);
});
