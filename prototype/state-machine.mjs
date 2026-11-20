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

function requireTrade(trade) {
  if (!trade || typeof trade !== "object" || typeof trade.id !== "string" ||
      !Object.values(TradeState).includes(trade.state) ||
      !Number.isSafeInteger(trade.requiredConfirmations) || trade.requiredConfirmations < 1) {
    fail("INVALID_TRADE", "trade aggregate is malformed");
  }
}

function next(trade, changes) {
  return Object.freeze({ ...trade, ...changes });
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

    case "REFUND_CONFIRMED":
      requireState(trade, [TradeState.DISPUTED], event.type);
      requireHash(event.refundTxid, "refundTxid");
      requireHash(event.blockHash, "blockHash");
      requirePositiveInteger(event.blockHeight, "blockHeight");
      requirePositiveInteger(event.confirmations, "confirmations");
      if (event.confirmations < trade.requiredConfirmations) {
        fail("REFUND_NOT_FINAL", "refund has not met the configured confirmation policy");
      }
      return next(trade, {
        state: TradeState.REFUNDED,
        refundTxid: event.refundTxid,
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
        state: TradeState.DISPUTED,
        refundTxid: null,
        refundBlockHash: null,
        refundBlockHeight: null,
        refundConfirmations: 0
      });

    default:
      fail("UNKNOWN_EVENT", "unsupported event type: " + event.type);
  }
}