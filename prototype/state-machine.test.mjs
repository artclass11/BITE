import test from "node:test";
import assert from "node:assert/strict";
import {
  TradeState,
  TradeTransitionError,
  createTrade,
  applyTradeEvent
} from "./state-machine.mjs";

const HASH_A = "a".repeat(64);
const HASH_B = "b".repeat(64);
const HASH_C = "c".repeat(64);
const HASH_D = "d".repeat(64);

function openTrade(requiredConfirmations = 3) {
  let trade = createTrade({ id: "trade-test-001", requiredConfirmations });
  trade = applyTradeEvent(trade, { type: "PUBLISH" });
  return trade;
}

function agreedTrade() {
  let trade = openTrade();
  trade = applyTradeEvent(trade, {
    type: "PROPOSE_MATCH", counterpartyId: "counterparty-01", termsHash: HASH_A
  });
  trade = applyTradeEvent(trade, { type: "ACCEPT_MATCH", termsHash: HASH_A });
  return trade;
}

function readyTrade() {
  let trade = agreedTrade();
  trade = applyTradeEvent(trade, { type: "PREPARE_PSBT", psbtHash: HASH_B });
  trade = applyTradeEvent(trade, {
    type: "PARTIAL_SIGNATURES_VERIFIED", psbtHash: HASH_B, signatureCount: 1
  });
  trade = applyTradeEvent(trade, {
    type: "SIGNATURE_POLICY_SATISFIED", psbtHash: HASH_B, signaturePolicyId: "test-policy-v1"
  });
  return trade;
}

function broadcastTrade() {
  return applyTradeEvent(readyTrade(), { type: "TRANSACTION_BROADCAST", txid: HASH_C });
}

function disputedTrade() {
  return applyTradeEvent(broadcastTrade(), {
    type: "RAISE_DISPUTE", disputeCode: "manual-dispute"
  });
}

function refundBroadcastTrade() {
  return applyTradeEvent(disputedTrade(), {
    type: "REFUND_BROADCAST", refundIntentHash: HASH_A, refundTxid: HASH_B
  });
}

test("a complete valid flow reaches SETTLED only at the configured confirmation threshold", () => {
  const original = broadcastTrade();
  const confirming = applyTradeEvent(original, {
    type: "TRANSACTION_CONFIRMED", txid: HASH_C, blockHash: HASH_D,
    blockHeight: 100, confirmations: 2
  });
  assert.equal(confirming.state, TradeState.CONFIRMING);
  const settled = applyTradeEvent(confirming, {
    type: "TRANSACTION_CONFIRMED", txid: HASH_C, blockHash: HASH_D,
    blockHeight: 100, confirmations: 3
  });
  assert.equal(settled.state, TradeState.SETTLED);
  assert.equal(settled.confirmations, 3);
  assert.equal(original.state, TradeState.BROADCAST, "transitions must not mutate the prior aggregate");
});

test("a match cannot skip order publication or proposal", () => {
  const trade = createTrade({ id: "trade-invalid" });
  assert.throws(
    () => applyTradeEvent(trade, { type: "ACCEPT_MATCH", termsHash: HASH_A }),
    (error) => error instanceof TradeTransitionError && error.code === "INVALID_STATE"
  );
});

test("a mismatched accepted order hash is rejected", () => {
  const trade = applyTradeEvent(openTrade(), {
    type: "PROPOSE_MATCH", counterpartyId: "counterparty-02", termsHash: HASH_A
  });
  assert.throws(
    () => applyTradeEvent(trade, { type: "ACCEPT_MATCH", termsHash: HASH_B }),
    (error) => error.code === "TERMS_MISMATCH"
  );
});

test("a signature for a different PSBT is rejected", () => {
  const trade = applyTradeEvent(agreedTrade(), { type: "PREPARE_PSBT", psbtHash: HASH_B });
  assert.throws(
    () => applyTradeEvent(trade, {
      type: "PARTIAL_SIGNATURES_VERIFIED", psbtHash: HASH_A, signatureCount: 1
    }),
    (error) => error.code === "PSBT_MISMATCH"
  );
});

test("invalid transaction hashes and zero confirmations are rejected", () => {
  const trade = broadcastTrade();
  assert.throws(
    () => applyTradeEvent(trade, {
      type: "TRANSACTION_CONFIRMED", txid: "not-a-txid", blockHash: HASH_D,
      blockHeight: 100, confirmations: 1
    }),
    (error) => error.code === "INVALID_EVENT"
  );
  assert.throws(
    () => applyTradeEvent(trade, {
      type: "TRANSACTION_CONFIRMED", txid: HASH_C, blockHash: HASH_D,
      blockHeight: 100, confirmations: 0
    }),
    (error) => error.code === "INVALID_EVENT"
  );
});

test("a reorg invalidates even a previously settled state", () => {
  let trade = applyTradeEvent(broadcastTrade(), {
    type: "TRANSACTION_CONFIRMED", txid: HASH_C, blockHash: HASH_D,
    blockHeight: 100, confirmations: 3
  });
  assert.equal(trade.state, TradeState.SETTLED);
  trade = applyTradeEvent(trade, { type: "REORG_OBSERVED", orphanedBlockHash: HASH_D });
  assert.equal(trade.state, TradeState.REORGED);
  assert.equal(trade.confirmations, 0);
  trade = applyTradeEvent(trade, { type: "MEMPOOL_RESEEN", txid: HASH_C });
  assert.equal(trade.state, TradeState.BROADCAST);
});

test("block changes and confirmation regressions require an explicit reorg event", () => {
  const confirming = applyTradeEvent(broadcastTrade(), {
    type: "TRANSACTION_CONFIRMED", txid: HASH_C, blockHash: HASH_D,
    blockHeight: 100, confirmations: 2
  });
  assert.throws(() => applyTradeEvent(confirming, {
    type: "TRANSACTION_CONFIRMED", txid: HASH_C, blockHash: HASH_A,
    blockHeight: 99, confirmations: 3
  }), (error) => error.code === "REORG_REQUIRED");
  assert.throws(() => applyTradeEvent(confirming, {
    type: "TRANSACTION_CONFIRMED", txid: HASH_C, blockHash: HASH_D,
    blockHeight: 100, confirmations: 1
  }), (error) => error.code === "CONFIRMATION_REGRESSION");
});

test("a conflicting spend moves the trade to DISPUTED", () => {
  const trade = applyTradeEvent(broadcastTrade(), {
    type: "TRANSACTION_CONFIRMED", txid: HASH_C, blockHash: HASH_D,
    blockHeight: 100, confirmations: 3
  });
  const disputed = applyTradeEvent(trade, {
    type: "CHAIN_CONFLICT", conflictingTxid: HASH_A, disputeCode: "conflicting-spend"
  });
  assert.equal(disputed.state, TradeState.DISPUTED);
});

test("an unmatched order may expire or be cancelled, but a matched trade cannot be cancelled unilaterally", () => {
  const open = openTrade();
  assert.equal(applyTradeEvent(open, { type: "EXPIRE" }).state, TradeState.EXPIRED);
  assert.equal(applyTradeEvent(open, { type: "CANCEL" }).state, TradeState.CANCELLED);
  const matched = applyTradeEvent(open, {
    type: "PROPOSE_MATCH", counterpartyId: "counterparty-03", termsHash: HASH_A
  });
  assert.throws(() => applyTradeEvent(matched, { type: "CANCEL" }), (error) => error.code === "INVALID_STATE");
});

test("a refund is not final until its own confirmation policy is met, and refund reorgs reopen the dispute", () => {
  let trade = applyTradeEvent(broadcastTrade(), {
    type: "RAISE_DISPUTE", disputeCode: "manual-dispute"
  });
  assert.throws(() => applyTradeEvent(trade, {
    type: "REFUND_CONFIRMED", refundTxid: HASH_A, blockHash: HASH_D,
    blockHeight: 102, confirmations: 2
  }), (error) => error.code === "REFUND_NOT_FINAL");
  trade = applyTradeEvent(trade, {
    type: "REFUND_CONFIRMED", refundTxid: HASH_A, blockHash: HASH_D,
    blockHeight: 102, confirmations: 3
  });
  assert.equal(trade.state, TradeState.REFUNDED);
  trade = applyTradeEvent(trade, { type: "REFUND_REORG_OBSERVED", orphanedBlockHash: HASH_D });
  assert.equal(trade.state, TradeState.DISPUTED);
  assert.equal(trade.refundTxid, null);
});

test("unknown events fail closed", () => {
  const trade = openTrade();
  assert.throws(
    () => applyTradeEvent(trade, { type: "MARK_AS_PAID_BY_WEBHOOK" }),
    (error) => error.code === "UNKNOWN_EVENT"
  );
});
test("refund must first be bound to a known intent and transaction", () => {
  const disputed = disputedTrade();
  assert.throws(() => applyTradeEvent(disputed, {
    type: "REFUND_CONFIRMED", refundIntentHash: HASH_A, refundTxid: HASH_B,
    blockHash: HASH_D, blockHeight: 102, confirmations: 3
  }), (error) => error.code === "INVALID_STATE");

  const refund = refundBroadcastTrade();
  assert.throws(() => applyTradeEvent(refund, {
    type: "REFUND_CONFIRMED", refundIntentHash: HASH_A, refundTxid: HASH_C,
    blockHash: HASH_D, blockHeight: 102, confirmations: 3
  }), (error) => error.code === "REFUND_MISMATCH");
  assert.throws(() => applyTradeEvent(refund, {
    type: "REFUND_CONFIRMED", refundIntentHash: HASH_D, refundTxid: HASH_B,
    blockHash: HASH_D, blockHeight: 102, confirmations: 3
  }), (error) => error.code === "REFUND_MISMATCH");
});

test("a refund is not final until its own confirmation policy is met, and refund reorgs reopen it", () => {
  let trade = refundBroadcastTrade();
  assert.throws(() => applyTradeEvent(trade, {
    type: "REFUND_CONFIRMED", refundIntentHash: HASH_A, refundTxid: HASH_B,
    blockHash: HASH_D, blockHeight: 102, confirmations: 2
  }), (error) => error.code === "REFUND_NOT_FINAL");
  trade = applyTradeEvent(trade, {
    type: "REFUND_CONFIRMED", refundIntentHash: HASH_A, refundTxid: HASH_B,
    blockHash: HASH_D, blockHeight: 102, confirmations: 3
  });
  assert.equal(trade.state, TradeState.REFUNDED);
  trade = applyTradeEvent(trade, { type: "REFUND_REORG_OBSERVED", orphanedBlockHash: HASH_D });
  assert.equal(trade.state, TradeState.REFUND_BROADCAST);
  assert.equal(trade.refundTxid, HASH_B, "refund identity remains bound after reorg");
  assert.equal(trade.refundBlockHash, null);
});

test("a settled trade cannot be forged from an inconsistent aggregate", () => {
  const forged = {
    ...createTrade({ id: "forged" }),
    state: TradeState.SETTLED,
    txid: HASH_C,
    confirmations: 1,
    blockHash: HASH_D,
    blockHeight: 100
  };
  assert.throws(
    () => applyTradeEvent(forged, { type: "RAISE_DISPUTE", disputeCode: "test" }),
    (error) => error.code === "INVALID_TRADE"
  );
});

test("a chain conflict cannot name the current trade transaction as its own conflict", () => {
  const trade = broadcastTrade();
  assert.throws(() => applyTradeEvent(trade, {
    type: "CHAIN_CONFLICT", conflictingTxid: HASH_C, disputeCode: "bad-conflict"
  }), (error) => error.code === "CONFLICT_TXID_MATCH");
});
