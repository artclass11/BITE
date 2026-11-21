# BITE PSBT intent v1 — regtest prototype

**Status:** experimental policy fixture; not a production transaction protocol.

## Scope

The bite-psbt crate adds bounded PSBT v0 parsing and a strict comparison between an unsigned PSBT and an application-supplied trade intent. It deliberately supports only:

- Transaction version 2, zero locktime, and final input sequences.
- At most 8 inputs and 8 outputs.
- Native P2WSH inputs with a deterministic 2-of-2 ECDSA multisig witness script.
- Two distinct compressed participant public keys per input.
- SIGHASH_ALL signatures only.
- Exact input outpoints and exact, ordered output script/value pairs.
- A declared absolute fee range with a 100,000-satoshi hard ceiling.
- PSBT payloads no larger than 100 KiB.

Taproot, legacy inputs, redeem scripts, finalized inputs, non-witness UTXO data, unknown or proprietary PSBT fields, non-standard sighash types, and unsupported transaction headers fail closed. PSBT v2 is not accepted.

## Intent binding

The caller supplies every expected input (outpoint, value and two signer public keys), every expected output (script and value), and minimum/maximum absolute fee. The validator compares the PSBT transaction to these fields in order. It rejects changed inputs, extra or reordered outputs, altered output amounts, mismatched P2WSH witness scripts, inconsistent witness_utxo metadata, unlisted signers and invalid ECDSA signatures.

Partial signatures are checked against the PSBT BIP143 sighash message. A result is marked ready for the test-only witness assembler only when each declared input has both expected signatures. The prototype test creates independent PSBT copies, signs each with a different deterministic fixture key, combines the partial signatures and assembles the expected CHECKMULTISIG witness.

## Critical limitations

1. A PSBT does not prove that an outpoint exists, remains unspent, carries the declared value, or belongs to the declared participant. witness_utxo is supplied metadata. A future implementation must resolve every prevout against verified chain data and a current UTXO view, including conflict handling.
2. A PSBT does not encode an authoritative Bitcoin network. The caller must independently ensure it is operating on regtest. Do not reuse this prototype as a mainnet policy gate.
3. The test assembles a witness but does not run an independent Bitcoin Script interpreter or broadcast the transaction to Bitcoin Core. It is a cryptographic partial-signature simulation, not a complete spend-validation test.
4. The fee control is an absolute satoshi cap, not a fee-rate estimator. Dust, relay policy, coin selection, input-weight estimation, change detection, locktime/refund policy and transaction replacement are not defined.
5. Test keys are deterministic disposable fixtures. No wallet, hardware signer, seed handling, user key storage, private-key service, custody, broadcast, deposit, withdrawal or real-funds path is added.
6. A valid Bitcoin transaction cannot prove off-chain USD payment. This work does not settle a BTC/USD trade or select a fiat payment rail.

The accepted production settlement model remains undecided. Independent protocol review and adversarial testing remain mandatory before handling value.
