/**
 * BITE prototype trade state machine.
 *
 * This module models allowed business-state transitions only. It does not
 * verify Nostr signatures, parse or validate PSBTs, observe Bitcoin consensus,
 * sign transactions, manage keys, or broadcast transactions. Production code
 * must call it only after independent protocol/cryptographic validation.
 */

export const TradeState = Object.freeze({
  DRAFT: "DRAFT",
  OPEN: "OPEN",
  MATCH_PROPOSED: "MATCH_PROPOSED",
  AGREED: "AGREED",
  PSBT_PREPARED: "PSBT_PREPARED",
  PARTIALLY_SIGNED: "PARTIALLY_SIGNED",
  READY_TO_BROADCAST: "READY_TO_BROADCAST",
  BROADCAST: "BROADCAST",
  CONFIRMING: "CONFIRMING",
  SETTLED: "SETTLED",
  REORGED: "REORGED",
  REFUND_BROADCAST: "REFUND_BROADCAST",
  EXPIRED: "EXPIRED",
  CANCELLED: "CANCELLED",
  DISPUTED: "DISPUTED",
  REFUNDED: "REFUNDED"
});

export class TradeTransitionError extends Error {
  constructor(code, message) {
    super(message);
    this.name = "TradeTransitionError";
    this.code = code;
  }
}

const HEX_32 = /^[0-9a-f]{64}$/;

function fail(code, message) {
  throw new TradeTransitionError(code, message);
}

function requireState(trade, allowed, eventType) {
  if (!allowed.includes(trade.state)) {
    fail("INVALID_STATE", eventType + " is not allowed from " + trade.state);
  }
}

function requireText(value, name) {
  if (typeof value !== "string" || value.trim().length === 0) {
    fail("INVALID_EVENT", name + " must be a non-empty string");
  }
}

function requireHash(value, name) {
  if (typeof value !== "string" || !HEX_32.test(value)) {
    fail("INVALID_EVENT", name + " must be 32-byte lowercase hex");
  }
}

function requirePositiveInteger(value, name) {
  if (!Number.isSafeInteger(value) || value < 1) {
    fail("INVALID_EVENT", name + " must be a positive safe integer");
  }
}

function requireNonNegativeInteger(value, name) {
  if (!Number.isSafeInteger(value) || value < 0) {
    fail("INVALID_TRADE", name + " must be a non-negative safe integer");
  }
}

function requireTrade(trade) {
  if (!trade || typeof trade !== "object" || Array.isArray(trade) ||
      typeof trade.id !== "string" || trade.id.trim().length === 0 ||
      !Object.values(TradeState).includes(trade.state) ||
      !Number.isSafeInteger(trade.requiredConfirmations) || trade.requiredConfirmations < 1) {
    fail("INVALID_TRADE", "trade aggregate is malformed");
  }

  requireNonNegativeInteger(trade.verifiedSignatureCount, "verifiedSignatureCount");
  requireNonNegativeInteger(trade.confirmations, "confirmations");
  requireNonNegativeInteger(trade.refundConfirmations, "refundConfirmations");

  for (const [field, value] of Object.entries({
    termsHash: trade.termsHash,
    psbtHash: trade.psbtHash,
    txid: trade.txid,
    blockHash: trade.blockHash,
    refundIntentHash: trade.refundIntentHash,
    refundTxid: trade.refundTxid,
    refundBlockHash: trade.refundBlockHash
  })) {
    if (value !== null) requireHash(value, field);
  }

  for (const [field, value] of Object.entries({
    blockHeight: trade.blockHeight,
    refundBlockHeight: trade.refundBlockHeight
  })) {
    if (value !== null) requirePositiveInteger(value, field);
  }

  for (const [field, value] of Object.entries({
    counterpartyId: trade.counterpartyId,
    signaturePolicyId: trade.signaturePolicyId,
    disputeCode: trade.disputeCode
  })) {
    if (value !== null && (typeof value !== "string" || value.trim().length === 0)) {
      fail("INVALID_TRADE", field + " must be null or a non-empty string");
    }
  }

  if ([TradeState.MATCH_PROPOSED, TradeState.AGREED].includes(trade.state) &&
      (!trade.counterpartyId || !trade.termsHash)) {
    fail("INVALID_TRADE", "matched trade is missing counterparty or terms hash");
  }
  if ([TradeState.PSBT_PREPARED, TradeState.PARTIALLY_SIGNED, TradeState.READY_TO_BROADCAST].includes(trade.state) &&
      !trade.psbtHash) {
    fail("INVALID_TRADE", "PSBT state is missing its PSBT hash");
  }
  if (trade.state === TradeState.PARTIALLY_SIGNED && trade.verifiedSignatureCount < 1) {
    fail("INVALID_TRADE", "partially-signed trade must contain a verified signature count");
  }
  if (trade.state === TradeState.READY_TO_BROADCAST && !trade.signaturePolicyId) {
    fail("INVALID_TRADE", "broadcast-ready trade is missing its satisfied signature policy");
  }
  if ([TradeState.BROADCAST, TradeState.CONFIRMING, TradeState.SETTLED, TradeState.REORGED].includes(trade.state) &&
      !trade.txid) {
    fail("INVALID_TRADE", "broadcast trade is missing its transaction ID");
  }
  if ([TradeState.CONFIRMING, TradeState.SETTLED].includes(trade.state) &&
      (!trade.blockHash || trade.blockHeight === null || trade.confirmations < 1)) {
    fail("INVALID_TRADE", "confirmed trade is missing block metadata");
  }
  if (trade.state === TradeState.SETTLED && trade.confirmations < trade.requiredConfirmations) {
    fail("INVALID_TRADE", "settled trade has not met its configured confirmation policy");
  }
  if ([TradeState.BROADCAST, TradeState.REORGED].includes(trade.state) &&
      (trade.blockHash !== null || trade.blockHeight !== null || trade.confirmations !== 0)) {
    fail("INVALID_TRADE", "unconfirmed trade contains stale block metadata");
  }
  if (trade.state === TradeState.DISPUTED && !trade.disputeCode) {
    fail("INVALID_TRADE", "disputed trade is missing a dispute code");
  }
  if ([TradeState.REFUND_BROADCAST, TradeState.REFUNDED].includes(trade.state) &&
      (!trade.refundIntentHash || !trade.refundTxid)) {
    fail("INVALID_TRADE", "refund state is missing its bound intent or transaction ID");
  }
  if (trade.state === TradeState.REFUND_BROADCAST &&
      (trade.refundBlockHash !== null || trade.refundBlockHeight !== null || trade.refundConfirmations !== 0)) {
    fail("INVALID_TRADE", "unconfirmed refund contains stale block metadata");
  }
  if (trade.state === TradeState.REFUNDED &&
      (!trade.refundBlockHash || trade.refundBlockHeight === null ||
       trade.refundConfirmations < trade.requiredConfirmations)) {
    fail("INVALID_TRADE", "refunded trade has not met its configured confirmation policy");
  }
}

function next(trade, changes) {
  const updated = Object.freeze({ ...trade, ...changes });
  requireTrade(updated);
  return updated;
}

export function createTrade({ id, requiredConfirmations = 6 } = {}) {
  requireText(id, "id");
  requirePositiveInteger(requiredConfirmations, "requiredConfirmations");
  return Object.freeze({
    id,
    state: TradeState.DRAFT,
    requiredConfirmations,
    counterpartyId: null,
    termsHash: null,
    psbtHash: null,
    verifiedSignatureCount: 0,
    signaturePolicyId: null,
    txid: null,
    blockHash: null,
    blockHeight: null,
    confirmations: 0,
    disputeCode: null,
    refundIntentHash: null,
    refundTxid: null,
    refundBlockHash: null,
    refundBlockHeight: null,
    refundConfirmations: 0
  });
}

/**
 * Apply one already-validated event and return a new immutable aggregate.
 * Caller authentication, authorization, signatures, PSBT parsing and chain
 * observations are deliberately outside this prototype's trust boundary.
 */
export function applyTradeEvent(trade, event) {
  requireTrade(trade);
  if (!event || typeof event !== "object" || typeof event.type !== "string") {
    fail("INVALID_EVENT", "event.type must be a string");
  }

  switch (event.type) {
    case "PUBLISH":
      requireState(trade, [TradeState.DRAFT], event.type);
      return next(trade, { state: TradeState.OPEN });

    case "PROPOSE_MATCH":
      requireState(trade, [TradeState.OPEN], event.type);
      requireText(event.counterpartyId, "counterpartyId");
      requireHash(event.termsHash, "termsHash");
      return next(trade, {
        state: TradeState.MATCH_PROPOSED,
        counterpartyId: event.counterpartyId,
        termsHash: event.termsHash
      });

    case "ACCEPT_MATCH":
      requireState(trade, [TradeState.MATCH_PROPOSED], event.type);
      requireHash(event.termsHash, "termsHash");
      if (event.termsHash !== trade.termsHash) {
        fail("TERMS_MISMATCH", "accepted terms do not match the proposed terms");
      }
      return next(trade, { state: TradeState.AGREED });

    case "PREPARE_PSBT":
      requireState(trade, [TradeState.AGREED], event.type);
      requireHash(event.psbtHash, "psbtHash");
      return next(trade, {
        state: TradeState.PSBT_PREPARED,
        psbtHash: event.psbtHash,
        verifiedSignatureCount: 0,
        signaturePolicyId: null
      });

    case "PARTIAL_SIGNATURES_VERIFIED":
      requireState(trade, [TradeState.PSBT_PREPARED, TradeState.PARTIALLY_SIGNED], event.type);
      requireHash(event.psbtHash, "psbtHash");
      if (event.psbtHash !== trade.psbtHash) {
        fail("PSBT_MISMATCH", "signature validation refers to a different PSBT");
      }
      requirePositiveInteger(event.signatureCount, "signatureCount");
      if (event.signatureCount <= trade.verifiedSignatureCount) {
        fail("SIGNATURE_COUNT_NOT_INCREASED", "verified signature count must increase");
      }
      return next(trade, {
        state: TradeState.PARTIALLY_SIGNED,
        verifiedSignatureCount: event.signatureCount
      });

    case "SIGNATURE_POLICY_SATISFIED":
      requireState(trade, [TradeState.PSBT_PREPARED, TradeState.PARTIALLY_SIGNED], event.type);
      requireHash(event.psbtHash, "psbtHash");
      requireText(event.signaturePolicyId, "signaturePolicyId");
      if (event.psbtHash !== trade.psbtHash) {
        fail("PSBT_MISMATCH", "signature policy refers to a different PSBT");
      }
      return next(trade, {
        state: TradeState.READY_TO_BROADCAST,
        signaturePolicyId: event.signaturePolicyId
      });

    case "TRANSACTION_BROADCAST":
      requireState(trade, [TradeState.READY_TO_BROADCAST], event.type);
      requireHash(event.txid, "txid");
      return next(trade, {
        state: TradeState.BROADCAST,
        txid: event.txid,
        blockHash: null,
        blockHeight: null,
        confirmations: 0
      });

    case "TRANSACTION_CONFIRMED": {
      requireState(trade, [TradeState.BROADCAST, TradeState.CONFIRMING, TradeState.SETTLED], event.type);
      requireHash(event.txid, "txid");
      requireHash(event.blockHash, "blockHash");
      requirePositiveInteger(event.blockHeight, "blockHeight");
      requirePositiveInteger(event.confirmations, "confirmations");
      if (event.txid !== trade.txid) {
        fail("TXID_MISMATCH", "confirmation is for a different transaction");
      }
      if (trade.blockHash !== null && event.blockHash !== trade.blockHash) {
        fail("REORG_REQUIRED", "a changed containing block must be reported as a reorganization first");
      }
      if (trade.blockHeight !== null && event.blockHeight !== trade.blockHeight) {
        fail("BLOCK_MISMATCH", "confirmation block height changed without a reorganization");
      }
      if (event.confirmations < trade.confirmations) {
        fail("CONFIRMATION_REGRESSION", "confirmation count cannot decrease without a reorganization");
      }
      const state = event.confirmations >= trade.requiredConfirmations
        ? TradeState.SETTLED
        : TradeState.CONFIRMING;
      return next(trade, {
        state,
        blockHash: event.blockHash,
        blockHeight: event.blockHeight,
        confirmations: event.confirmations
      });
    }

    case "REORG_OBSERVED":
      requireState(trade, [TradeState.CONFIRMING, TradeState.SETTLED], event.type);
      requireHash(event.orphanedBlockHash, "orphanedBlockHash");
      if (event.orphanedBlockHash !== trade.blockHash) {
        fail("BLOCK_MISMATCH", "reorg does not match the block currently recorded for this trade");
      }
      return next(trade, {
        state: TradeState.REORGED,
        blockHash: null,
        blockHeight: null,
        confirmations: 0
      });

    case "MEMPOOL_RESEEN":
      requireState(trade, [TradeState.REORGED], event.type);
      requireHash(event.txid, "txid");
      if (event.txid !== trade.txid) {
        fail("TXID_MISMATCH", "mempool observation is for a different transaction");
      }
      return next(trade, { state: TradeState.BROADCAST });

    case "CHAIN_CONFLICT":
      requireState(trade, [TradeState.BROADCAST, TradeState.CONFIRMING, TradeState.SETTLED, TradeState.REORGED], event.type);
      requireHash(event.conflictingTxid, "conflictingTxid");
      if (event.conflictingTxid === trade.txid) {
        fail("CONFLICT_TXID_MATCH", "conflicting transaction cannot equal the trade transaction");
      }
      requireText(event.disputeCode, "disputeCode");
      return next(trade, { state: TradeState.DISPUTED, disputeCode: event.disputeCode });

    case "RAISE_DISPUTE":
      requireState(trade, [TradeState.MATCH_PROPOSED, TradeState.AGREED, TradeState.PSBT_PREPARED,
        TradeState.PARTIALLY_SIGNED, TradeState.READY_TO_BROADCAST, TradeState.BROADCAST,
        TradeState.CONFIRMING, TradeState.REORGED], event.type);
      requireText(event.disputeCode, "disputeCode");
      return next(trade, { state: TradeState.DISPUTED, disputeCode: event.disputeCode });

    case "EXPIRE":
      requireState(trade, [TradeState.OPEN, TradeState.MATCH_PROPOSED], event.type);
      return next(trade, { state: TradeState.EXPIRED });

    case "CANCEL":
      requireState(trade, [TradeState.DRAFT, TradeState.OPEN], event.type);
      return next(trade, { state: TradeState.CANCELLED });

    case "REFUND_BROADCAST":
      requireState(trade, [TradeState.DISPUTED], event.type);
      requireHash(event.refundIntentHash, "refundIntentHash");
      requireHash(event.refundTxid, "refundTxid");
      return next(trade, {
        state: TradeState.REFUND_BROADCAST,
        refundIntentHash: event.refundIntentHash,
        refundTxid: event.refundTxid,
        refundBlockHash: null,
        refundBlockHeight: null,
        refundConfirmations: 0
      });

    case "REFUND_CONFIRMED":
      requireState(trade, [TradeState.REFUND_BROADCAST], event.type);
      requireHash(event.refundIntentHash, "refundIntentHash");
      requireHash(event.refundTxid, "refundTxid");
      requireHash(event.blockHash, "blockHash");
      requirePositiveInteger(event.blockHeight, "blockHeight");
      requirePositiveInteger(event.confirmations, "confirmations");
      if (event.refundIntentHash !== trade.refundIntentHash || event.refundTxid !== trade.refundTxid) {
        fail("REFUND_MISMATCH", "confirmed refund does not match the previously bound refund intent and transaction");
      }
      if (event.confirmations < trade.requiredConfirmations) {
        fail("REFUND_NOT_FINAL", "refund has not met the configured confirmation policy");
      }
      return next(trade, {
        state: TradeState.REFUNDED,
        refundBlockHash: event.blockHash,
        refundBlockHeight: event.blockHeight,
        refundConfirmations: event.confirmations
      });

    case "REFUND_REORG_OBSERVED":
      requireState(trade, [TradeState.REFUNDED], event.type);
      requireHash(event.orphanedBlockHash, "orphanedBlockHash");
      if (event.orphanedBlockHash !== trade.refundBlockHash) {
        fail("BLOCK_MISMATCH", "refund reorg does not match the recorded refund block");
      }
      return next(trade, {
        state: TradeState.REFUND_BROADCAST,
        refundBlockHash: null,
        refundBlockHeight: null,
        refundConfirmations: 0
      });

    default:
      fail("UNKNOWN_EVENT", "unsupported event type: " + event.type);
  }
}