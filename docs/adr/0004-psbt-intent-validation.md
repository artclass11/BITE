# ADR 0004 — Strict PSBT intent-validation prototype

- **Status:** Accepted for limited regtest experimentation only
- **Date:** 2026-11-21
- **Scope:** Bounded PSBT v0 parsing, exact transaction-intent comparison and simulated 2-of-2 partial signing

## Context

The order protocol and read-only Bitcoin observer did not parse PSBTs or compare a proposed transaction against an agreed trade intent. The prototype needs a deterministic way to reject simple transaction substitution before any future user-controlled signing flow is designed.

## Decision

1. Add a separate bite-psbt Rust crate pinned to the already-used bitcoin = 0.32.102 dependency version.
2. Bound raw PSBT input to 100 KiB; accept PSBT v0 only.
3. Limit policy fixtures to at most 8 inputs and 8 outputs, transaction version 2, zero locktime and final sequences.
4. Require exact, ordered input outpoints and exact, ordered output scripts and satoshi amounts. Extra, missing, reordered or substituted values fail.
5. Require each input's declared witness_utxo to match the intent amount and native P2WSH script derived from exactly two distinct compressed signer keys.
6. Require the witness script to be the deterministic 2-of-2 CHECKMULTISIG script. Only ECDSA SIGHASH_ALL is accepted.
7. Verify every partial signature against the transaction's PSBT sighash message and the key it claims to represent. Unknown signers or invalid signatures fail closed.
8. Reject Taproot, non-witness UTXO data, redeem scripts, finalized script fields, unknown/proprietary PSBT keys and unsupported preimage fields.
9. Bound the absolute fee to the declared intent range and a hard prototype maximum of 100,000 satoshis.
10. Test two independent signing participants by signing separate copies of one PSBT with deterministic disposable fixture keys, combining partial signatures and assembling the expected witness in the test module only.

## Explicit trust boundaries and limitations

- witness_utxo is not proof of a real prevout. The validator does not prove that an outpoint exists, remains unspent, has the declared amount, or is controlled by either participant.
- PSBT does not contain an authoritative network identifier. The caller must independently establish that it is operating in regtest. This crate does not provide a mainnet safety gate.
- The test assembles a witness but does not run an independent Bitcoin Script interpreter or broadcast a spend to Bitcoin Core.
- The absolute fee cap does not implement fee-rate estimation, dust policy, relay policy, coin selection, change detection or replacement policy.
- The crate contains no private-key import, wallet, hardware-signer, finalization API, server signing, custody, transaction broadcast, deposit, withdrawal or real-money functionality.
- Bitcoin consensus cannot prove that a counterparty made an off-chain USD payment.

## Consequences

BITE now has a bounded PSBT parser and deterministic intent checker suitable for adversarial unit tests. A separate chain-verified prevout/UTXO layer, Script-engine verification, a complete regtest spend-acceptance test, persistent observer recovery, wallet/hardware signer integration, protocol review and a finalized USD settlement model are still required. This ADR does not approve mainnet, custody or customer funds.
