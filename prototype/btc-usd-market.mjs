/**
 * BITE BTC/USD order-domain primitives.
 *
 * Integer-only prototype: no JavaScript floating point is used for monetary
 * values. This module validates a public order payload and computes exact
 * quote-value rationals. It does NOT verify Nostr signatures, authorize the
 * maker, match orders, determine a USD payment rail, or settle a trade.
 */

export const MARKET = "BTC-USD";
export const SATOSHIS_PER_BTC = 100_000_000n;
export const MAX_BITCOIN_SUPPLY_SATS = 2_100_000_000_000_000n;
export const DEFAULT_MAX_ORDER_TTL_SECONDS = 7 * 24 * 60 * 60;

const HEX_32_BYTES = /^[0-9a-f]{64}$/;
const CANONICAL_UINT = /^(0|[1-9][0-9]*)$/;
const MAX_PRICE_MICRO_USD_PER_BTC = 999_999_999_999_999_999n;
const ALLOWED_FIELDS = new Set([
  "version",
  "market",
  "orderId",
  "makerPubkey",
  "side",
  "priceUsdMicrosPerBtc",
  "sizeSats",
  "createdAt",
  "expiresAt"
]);

export class OrderValidationError extends Error {
  constructor(code, message) {
    super(message);
    this.name = "OrderValidationError";
    this.code = code;
  }
}

function reject(code, message) {
  throw new OrderValidationError(code, message);
}

function parseCanonicalUint(value, field, max) {
  if (typeof value !== "string" || !CANONICAL_UINT.test(value)) {
    reject("NON_CANONICAL_INTEGER", field + " must be a canonical unsigned decimal string");
  }
  const parsed = BigInt(value);
  if (parsed <= 0n) reject("OUT_OF_RANGE", field + " must be greater than zero");
  if (parsed > max) reject("OUT_OF_RANGE", field + " exceeds the permitted maximum");
  return parsed;
}

function requireSafeTimestamp(value, field) {
  if (!Number.isSafeInteger(value) || value < 0) {
    reject("INVALID_TIMESTAMP", field + " must be a non-negative safe integer");
  }
}

export function validateBtcUsdOrder(order, {
  nowSeconds,
  maxTtlSeconds = DEFAULT_MAX_ORDER_TTL_SECONDS,
  allowedFutureClockSkewSeconds = 30
} = {}) {
  if (!order || typeof order !== "object" || Array.isArray(order)) {
    reject("INVALID_ORDER", "order must be an object");
  }

  for (const key of Object.keys(order)) {
    if (!ALLOWED_FIELDS.has(key)) reject("UNKNOWN_FIELD", "unsupported order field: " + key);
  }

  const required = [...ALLOWED_FIELDS];
  for (const key of required) {
    if (!Object.hasOwn(order, key)) reject("MISSING_FIELD", "missing order field: " + key);
  }

  if (order.version !== 1) reject("UNSUPPORTED_VERSION", "only order schema version 1 is supported");
  if (order.market !== MARKET) reject("UNSUPPORTED_MARKET", "only BTC-USD is supported");

  if (typeof order.orderId !== "string" || !HEX_32_BYTES.test(order.orderId)) {
    reject("INVALID_ORDER_ID", "orderId must be 32-byte lowercase hex");
  }
  if (typeof order.makerPubkey !== "string" || !HEX_32_BYTES.test(order.makerPubkey)) {
    reject("INVALID_PUBKEY", "makerPubkey must be 32-byte lowercase hex");
  }
  if (order.side !== "BUY" && order.side !== "SELL") {
    reject("INVALID_SIDE", "side must be BUY or SELL");
  }

  const price = parseCanonicalUint(
    order.priceUsdMicrosPerBtc,
    "priceUsdMicrosPerBtc",
    MAX_PRICE_MICRO_USD_PER_BTC
  );
  const size = parseCanonicalUint(
    order.sizeSats,
    "sizeSats",
    MAX_BITCOIN_SUPPLY_SATS
  );

  requireSafeTimestamp(order.createdAt, "createdAt");
  requireSafeTimestamp(order.expiresAt, "expiresAt");

  if (!Number.isSafeInteger(nowSeconds) || nowSeconds < 0) {
    reject("INVALID_CLOCK", "nowSeconds must be supplied as a non-negative safe integer");
  }
  if (!Number.isSafeInteger(maxTtlSeconds) || maxTtlSeconds <= 0) {
    reject("INVALID_POLICY", "maxTtlSeconds must be a positive safe integer");
  }
  if (!Number.isSafeInteger(allowedFutureClockSkewSeconds) || allowedFutureClockSkewSeconds < 0) {
    reject("INVALID_POLICY", "allowedFutureClockSkewSeconds must be a non-negative safe integer");
  }

  if (order.expiresAt <= order.createdAt) {
    reject("INVALID_EXPIRY", "expiresAt must be later than createdAt");
  }
  if (order.expiresAt - order.createdAt > maxTtlSeconds) {
    reject("EXPIRY_TOO_FAR", "order expiry exceeds the configured maximum lifetime");
  }
  if (order.createdAt > nowSeconds + allowedFutureClockSkewSeconds) {
    reject("CREATED_IN_FUTURE", "order creation time is too far in the future");
  }
  if (order.expiresAt <= nowSeconds) {
    reject("ORDER_EXPIRED", "order has expired");
  }

  return Object.freeze({
    version: 1,
    market: MARKET,
    orderId: order.orderId,
    makerPubkey: order.makerPubkey,
    side: order.side,
    priceUsdMicrosPerBtc: price.toString(),
    sizeSats: size.toString(),
    createdAt: order.createdAt,
    expiresAt: order.expiresAt
  });
}

/**
 * Return the exact quote notional as a rational number of micro-USD:
 *
 *     (priceUsdMicrosPerBtc * sizeSats) / 100_000_000
 *
 * No rounding is performed. A future settlement specification must define
 * the USD payment rounding and precision policy before a trade can settle.
 */
export function calculateQuoteMicroUsdRational({ priceUsdMicrosPerBtc, sizeSats }) {
  const price = parseCanonicalUint(
    priceUsdMicrosPerBtc,
    "priceUsdMicrosPerBtc",
    MAX_PRICE_MICRO_USD_PER_BTC
  );
  const size = parseCanonicalUint(sizeSats, "sizeSats", MAX_BITCOIN_SUPPLY_SATS);
  return Object.freeze({
    numerator: (price * size).toString(),
    denominator: SATOSHIS_PER_BTC.toString(),
    unit: "microUSD"
  });
}

export function compareBtcUsdPrices(left, right) {
  const a = parseCanonicalUint(
    left,
    "left price",
    MAX_PRICE_MICRO_USD_PER_BTC
  );
  const b = parseCanonicalUint(
    right,
    "right price",
    MAX_PRICE_MICRO_USD_PER_BTC
  );
  return a < b ? -1 : a > b ? 1 : 0;
}
