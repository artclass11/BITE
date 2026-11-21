//! Narrow PSBT v0 policy checks for BITE's regtest prototype.
//!
//! This crate does not create keys, construct transactions, contact Bitcoin
//! Core, prove that prevouts are unspent, finalize transactions, or broadcast.
//! PSBTs do not contain an authoritative Bitcoin network identifier; callers
//! must independently establish the regtest environment.
//!
//! Supported input policy: native P2WSH, exactly 2-of-2 ECDSA multisig,
//! compressed participant keys, and SIGHASH_ALL. Unsupported fields fail closed.

use bitcoin::{
    absolute::LockTime,
    ecdsa,
    opcodes::all::OP_CHECKMULTISIG,
    psbt::Psbt,
    script::Builder,
    secp256k1::Secp256k1,
    sighash::{EcdsaSighashType, SighashCache},
    transaction::Version,
    Amount, OutPoint, PublicKey, ScriptBuf, Sequence, TxOut,
};
use std::collections::HashSet;
use std::fmt;

pub const MAX_PSBT_BYTES: usize = 100 * 1024;
pub const MAX_POLICY_INPUTS: usize = 8;
pub const MAX_POLICY_OUTPUTS: usize = 8;
pub const MAX_POLICY_FEE_SATS: u64 = 100_000;
const MAX_MONEY_SATS: u64 = 2_100_000_000_000_000;

/// A prevout that must appear at the same index in the unsigned transaction.
/// The spending condition is a 2-of-2 P2WSH script derived from signers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedInput {
    pub outpoint: OutPoint,
    pub value_sats: u64,
    pub signers: [PublicKey; 2],
}

/// An exact transaction output. Additional outputs are rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedOutput {
    pub script_pubkey: ScriptBuf,
    pub value_sats: u64,
}

/// The fixed transaction intent agreed by both counterparties before signing.
///
/// This structure is not itself an authenticated exchange contract. The caller
/// must bind it to independently authenticated trade terms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegtestTradeIntent {
    pub inputs: Vec<ExpectedInput>,
    pub outputs: Vec<ExpectedOutput>,
    pub min_fee_sats: u64,
    pub max_fee_sats: u64,
}

/// Deterministic result of the intent comparison and partial-signature checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidationReport {
    pub input_total_sats: u64,
    pub output_total_sats: u64,
    pub fee_sats: u64,
    pub verified_signatures: usize,
    pub required_signatures: usize,
    pub ready_for_two_party_finalize: bool,
}

/// Data-free failures avoid accidentally logging wallet metadata or PSBT bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PsbtPolicyError {
    PsbtTooLarge,
    MalformedPsbt,
    InvalidIntent,
    DuplicateInput,
    UnsupportedPsbtVersion,
    InvalidPsbtShape,
    UnsupportedTransactionHeader,
    TransactionNotUnsigned,
    InputOrderMismatch,
    OutputMismatch,
    MissingWitnessUtxo,
    PrevoutMismatch,
    WitnessScriptMismatch,
    UnsupportedPsbtField,
    UnsupportedSighash,
    UnexpectedSigner,
    InvalidSignature,
    AmountOverflow,
    FeeUnderflow,
    FeeOutsidePolicy,
}

impl fmt::Display for PsbtPolicyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PSBT rejected by BITE prototype policy: {self:?}")
    }
}

impl std::error::Error for PsbtPolicyError {}

/// Parse a PSBT after applying a strict byte-size ceiling.
///
/// Parsing alone does not validate a transaction's intent, ownership, network,
/// or settlement status. Call validate_regtest_psbt after parsing.
pub fn parse_psbt_bounded(raw: &[u8]) -> Result<Psbt, PsbtPolicyError> {
    if raw.len() > MAX_PSBT_BYTES {
        return Err(PsbtPolicyError::PsbtTooLarge);
    }
    Psbt::deserialize(raw).map_err(|_| PsbtPolicyError::MalformedPsbt)
}

/// Compare an unsigned PSBT against an explicit trade intent.
///
/// Policy:
/// - PSBT v0 only; at most 8 inputs and 8 outputs.
/// - Version 2, zero locktime, final input sequences.
/// - Exact ordered input outpoints and output script/value pairs.
/// - Native P2WSH inputs with the declared 2-of-2 compressed signer keys.
/// - Matching witness_utxo metadata and the exact expected witness script.
/// - ECDSA SIGHASH_ALL only; every supplied partial signature is verified.
/// - No Taproot, legacy prevouts, finalized scripts, unknown or proprietary keys.
/// - The intent fee range is applied with a 100,000-sat hard ceiling.
///
/// IMPORTANT: witness_utxo is counterparty-provided metadata. This function does
/// not prove that an outpoint exists, is unspent, has the declared value, or is
/// controlled by either signer. A separate verified UTXO layer is required.
pub fn validate_regtest_psbt(
    psbt: &Psbt,
    intent: &RegtestTradeIntent,
) -> Result<ValidationReport, PsbtPolicyError> {
    if intent.inputs.is_empty()
        || intent.inputs.len() > MAX_POLICY_INPUTS
        || intent.outputs.is_empty()
        || intent.outputs.len() > MAX_POLICY_OUTPUTS
        || intent.min_fee_sats > intent.max_fee_sats
        || intent.max_fee_sats > MAX_POLICY_FEE_SATS
    {
        return Err(PsbtPolicyError::InvalidIntent);
    }

    let mut seen_outpoints = HashSet::with_capacity(intent.inputs.len());
    for expected in &intent.inputs {
        if !seen_outpoints.insert(expected.outpoint) {
            return Err(PsbtPolicyError::DuplicateInput);
        }
        if expected.value_sats == 0
            || expected.value_sats > MAX_MONEY_SATS
            || expected.signers[0] == expected.signers[1]
            || !expected.signers[0].compressed
            || !expected.signers[1].compressed
        {
            return Err(PsbtPolicyError::InvalidIntent);
        }
    }

    for expected in &intent.outputs {
        if expected.value_sats == 0 || expected.value_sats > MAX_MONEY_SATS {
            return Err(PsbtPolicyError::InvalidIntent);
        }
    }

    if psbt.version != 0 {
        return Err(PsbtPolicyError::UnsupportedPsbtVersion);
    }
    if !psbt.xpub.is_empty() || !psbt.proprietary.is_empty() || !psbt.unknown.is_empty() {
        return Err(PsbtPolicyError::UnsupportedPsbtField);
    }

    let tx = &psbt.unsigned_tx;
    if tx.input.len() != intent.inputs.len()
        || tx.output.len() != intent.outputs.len()
        || psbt.inputs.len() != tx.input.len()
        || psbt.outputs.len() != tx.output.len()
    {
        return Err(PsbtPolicyError::InvalidPsbtShape);
    }
    if tx.version != Version::TWO || tx.lock_time != LockTime::ZERO {
        return Err(PsbtPolicyError::UnsupportedTransactionHeader);
    }
    for txin in &tx.input {
        if !txin.script_sig.is_empty() || !txin.witness.is_empty() {
            return Err(PsbtPolicyError::TransactionNotUnsigned);
        }
        if txin.sequence != Sequence::MAX {
            return Err(PsbtPolicyError::UnsupportedTransactionHeader);
        }
    }

    let secp = Secp256k1::verification_only();
    let mut input_total_sats = 0u64;
    let mut verified_signatures = 0usize;

    for (index, expected) in intent.inputs.iter().enumerate() {
        let txin = &tx.input[index];
        let psbt_input = &psbt.inputs[index];

        if txin.previous_output != expected.outpoint {
            return Err(PsbtPolicyError::InputOrderMismatch);
        }

        if psbt_input.non_witness_utxo.is_some()
            || psbt_input.redeem_script.is_some()
            || psbt_input.final_script_sig.is_some()
            || psbt_input.final_script_witness.is_some()
            || !psbt_input.bip32_derivation.is_empty()
            || !psbt_input.ripemd160_preimages.is_empty()
            || !psbt_input.sha256_preimages.is_empty()
            || !psbt_input.hash160_preimages.is_empty()
            || !psbt_input.hash256_preimages.is_empty()
            || psbt_input.tap_key_sig.is_some()
            || !psbt_input.tap_script_sigs.is_empty()
            || !psbt_input.tap_scripts.is_empty()
            || !psbt_input.tap_key_origins.is_empty()
            || psbt_input.tap_internal_key.is_some()
            || psbt_input.tap_merkle_root.is_some()
            || !psbt_input.proprietary.is_empty()
            || !psbt_input.unknown.is_empty()
        {
            return Err(PsbtPolicyError::UnsupportedPsbtField);
        }

        let witness_script = expected_2of2_witness_script(&expected.signers)?;
        if psbt_input.witness_script.as_ref() != Some(&witness_script) {
            return Err(PsbtPolicyError::WitnessScriptMismatch);
        }

        let expected_prevout = TxOut {
            value: Amount::from_sat(expected.value_sats),
            script_pubkey: ScriptBuf::new_p2wsh(&witness_script.wscript_hash()),
        };
        let witness_utxo = psbt_input
            .witness_utxo
            .as_ref()
            .ok_or(PsbtPolicyError::MissingWitnessUtxo)?;
        if witness_utxo != &expected_prevout {
            return Err(PsbtPolicyError::PrevoutMismatch);
        }

        input_total_sats = input_total_sats
            .checked_add(expected.value_sats)
            .ok_or(PsbtPolicyError::AmountOverflow)?;
        if input_total_sats > MAX_MONEY_SATS {
            return Err(PsbtPolicyError::AmountOverflow);
        }

        if psbt_input
            .ecdsa_hash_ty()
            .map_err(|_| PsbtPolicyError::UnsupportedSighash)?
            != EcdsaSighashType::All
        {
            return Err(PsbtPolicyError::UnsupportedSighash);
        }

        if !psbt_input.partial_sigs.is_empty() {
            let mut cache = SighashCache::new(tx.clone());
            let (message, sighash_type) = psbt
                .sighash_ecdsa(index, &mut cache)
                .map_err(|_| PsbtPolicyError::UnsupportedSighash)?;
            if sighash_type != EcdsaSighashType::All {
                return Err(PsbtPolicyError::UnsupportedSighash);
            }

            for (public_key, signature) in &psbt_input.partial_sigs {
                if !expected.signers.contains(public_key) {
                    return Err(PsbtPolicyError::UnexpectedSigner);
                }
                if signature.sighash_type != EcdsaSighashType::All {
                    return Err(PsbtPolicyError::UnsupportedSighash);
                }
                secp.verify_ecdsa(&message, &signature.signature, &public_key.inner)
                    .map_err(|_| PsbtPolicyError::InvalidSignature)?;
                verified_signatures = verified_signatures
                    .checked_add(1)
                    .ok_or(PsbtPolicyError::AmountOverflow)?;
            }
        }
    }

    let mut output_total_sats = 0u64;
    for (actual, expected) in tx.output.iter().zip(&intent.outputs) {
        if actual.value.to_sat() != expected.value_sats
            || actual.script_pubkey != expected.script_pubkey
        {
            return Err(PsbtPolicyError::OutputMismatch);
        }
        output_total_sats = output_total_sats
            .checked_add(actual.value.to_sat())
            .ok_or(PsbtPolicyError::AmountOverflow)?;
        if output_total_sats > MAX_MONEY_SATS {
            return Err(PsbtPolicyError::AmountOverflow);
        }
    }

    for psbt_output in &psbt.outputs {
        if psbt_output.redeem_script.is_some()
            || psbt_output.witness_script.is_some()
            || !psbt_output.bip32_derivation.is_empty()
            || psbt_output.tap_internal_key.is_some()
            || psbt_output.tap_tree.is_some()
            || !psbt_output.tap_key_origins.is_empty()
            || !psbt_output.proprietary.is_empty()
            || !psbt_output.unknown.is_empty()
        {
            return Err(PsbtPolicyError::UnsupportedPsbtField);
        }
    }

    let fee_sats = input_total_sats
        .checked_sub(output_total_sats)
        .ok_or(PsbtPolicyError::FeeUnderflow)?;
    if fee_sats < intent.min_fee_sats || fee_sats > intent.max_fee_sats {
        return Err(PsbtPolicyError::FeeOutsidePolicy);
    }

    let required_signatures = intent.inputs.len() * 2;
    Ok(ValidationReport {
        input_total_sats,
        output_total_sats,
        fee_sats,
        verified_signatures,
        required_signatures,
        ready_for_two_party_finalize: verified_signatures == required_signatures,
    })
}

fn expected_2of2_witness_script(signers: &[PublicKey; 2]) -> Result<ScriptBuf, PsbtPolicyError> {
    if signers[0] == signers[1] || !signers[0].compressed || !signers[1].compressed {
        return Err(PsbtPolicyError::InvalidIntent);
    }

    Ok(Builder::new()
        .push_int(2)
        .push_key(&signers[0])
        .push_key(&signers[1])
        .push_int(2)
        .push_opcode(OP_CHECKMULTISIG)
        .into_script())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::{
        ecdsa,
        hashes::Hash,
        opcodes::all::OP_TRUE,
        psbt::PsbtSighashType,
        secp256k1::{PublicKey as SecpPublicKey, SecretKey},
        transaction::Version,
        TxIn, Txid, Witness,
    };

    fn fixture() -> (
        Psbt,
        RegtestTradeIntent,
        SecretKey,
        SecretKey,
        PublicKey,
        PublicKey,
        ScriptBuf,
    ) {
        let secp = Secp256k1::new();
        let maker_secret = SecretKey::from_slice(&[1u8; 32]).expect("fixture key");
        let taker_secret = SecretKey::from_slice(&[2u8; 32]).expect("fixture key");
        let maker_key = PublicKey::new(SecpPublicKey::from_secret_key(&secp, &maker_secret));
        let taker_key = PublicKey::new(SecpPublicKey::from_secret_key(&secp, &taker_secret));
        let signers = [maker_key, taker_key];
        let witness_script = expected_2of2_witness_script(&signers).expect("2-of-2 script");
        let prevout_script = ScriptBuf::new_p2wsh(&witness_script.wscript_hash());
        let outpoint = OutPoint::new(Txid::from_byte_array([42u8; 32]), 0);
        let output_script = Builder::new().push_opcode(OP_TRUE).into_script();

        let unsigned_tx = bitcoin::Transaction {
            version: Version::TWO,
            lock_time: LockTime::ZERO,
            input: vec![TxIn {
                previous_output: outpoint,
                script_sig: ScriptBuf::new(),
                sequence: Sequence::MAX,
                witness: Witness::new(),
            }],
            output: vec![TxOut {
                value: Amount::from_sat(99_000),
                script_pubkey: output_script.clone(),
            }],
        };

        let mut psbt = Psbt::from_unsigned_tx(unsigned_tx).expect("unsigned PSBT");
        psbt.inputs[0].witness_utxo = Some(TxOut {
            value: Amount::from_sat(100_000),
            script_pubkey: prevout_script,
        });
        psbt.inputs[0].witness_script = Some(witness_script.clone());

        let intent = RegtestTradeIntent {
            inputs: vec![ExpectedInput {
                outpoint,
                value_sats: 100_000,
                signers,
            }],
            outputs: vec![ExpectedOutput {
                script_pubkey: output_script,
                value_sats: 99_000,
            }],
            min_fee_sats: 1,
            max_fee_sats: 2_000,
        };

        (
            psbt,
            intent,
            maker_secret,
            taker_secret,
            maker_key,
            taker_key,
            witness_script,
        )
    }

    fn sign_copy(original: &Psbt, secret: &SecretKey, map_key: Option<PublicKey>) -> Psbt {
        let secp = Secp256k1::new();
        let mut signed = original.clone();
        let tx = signed.unsigned_tx.clone();
        let mut cache = SighashCache::new(tx);
        let (message, sighash_type) = signed.sighash_ecdsa(0, &mut cache).expect("PSBT sighash");
        let signature = secp.sign_ecdsa(&message, secret);
        let derived_key = PublicKey::new(SecpPublicKey::from_secret_key(&secp, secret));
        signed.inputs[0].partial_sigs.insert(
            map_key.unwrap_or(derived_key),
            ecdsa::Signature {
                signature,
                sighash_type,
            },
        );
        signed
    }

    #[test]
    fn bounded_parser_round_trips_psbt_v0() {
        let (psbt, _, _, _, _, _, _) = fixture();
        let parsed = parse_psbt_bounded(&psbt.serialize()).expect("parse fixture PSBT");
        assert_eq!(parsed, psbt);
    }

    #[test]
    fn parser_rejects_oversized_and_malformed_data() {
        assert_eq!(
            parse_psbt_bounded(&vec![0u8; MAX_PSBT_BYTES + 1]),
            Err(PsbtPolicyError::PsbtTooLarge)
        );
        assert_eq!(
            parse_psbt_bounded(b"not a PSBT"),
            Err(PsbtPolicyError::MalformedPsbt)
        );
    }

    #[test]
    fn validates_exact_unsigned_intent_before_any_signature() {
        let (psbt, intent, _, _, _, _, _) = fixture();
        let report = validate_regtest_psbt(&psbt, &intent).expect("valid intent");
        assert_eq!(report.input_total_sats, 100_000);
        assert_eq!(report.output_total_sats, 99_000);
        assert_eq!(report.fee_sats, 1_000);
        assert_eq!(report.verified_signatures, 0);
        assert_eq!(report.required_signatures, 2);
        assert!(!report.ready_for_two_party_finalize);
    }

    #[test]
    fn simulates_two_independent_signers_combining_partial_signatures() {
        let (psbt, intent, maker_secret, taker_secret, maker_key, taker_key, witness_script) =
            fixture();

        let mut maker_signed = sign_copy(&psbt, &maker_secret, None);
        let taker_signed = sign_copy(&psbt, &taker_secret, None);
        maker_signed
            .combine(taker_signed)
            .expect("combine signatures");

        let report = validate_regtest_psbt(&maker_signed, &intent).expect("both signatures valid");
        assert_eq!(report.verified_signatures, 2);
        assert!(report.ready_for_two_party_finalize);

        // Final witness assembly is exercised only in this deterministic fixture.
        let mut witness = Witness::new();
        witness.push(Vec::<u8>::new());
        for signer in [maker_key, taker_key] {
            let signature = maker_signed.inputs[0]
                .partial_sigs
                .get(&signer)
                .expect("required signature");
            witness.push(signature.to_vec());
        }
        witness.push(witness_script.as_bytes());
        let mut final_tx = maker_signed.unsigned_tx.clone();
        final_tx.input[0].witness = witness;
        assert_eq!(final_tx.input[0].witness.len(), 4);
    }

    #[test]
    fn rejects_input_substitution() {
        let (mut psbt, intent, _, _, _, _, _) = fixture();
        psbt.unsigned_tx.input[0].previous_output.vout = 1;
        assert_eq!(
            validate_regtest_psbt(&psbt, &intent),
            Err(PsbtPolicyError::InputOrderMismatch)
        );
    }

    #[test]
    fn rejects_output_value_or_script_substitution() {
        let (mut psbt, intent, _, _, _, _, _) = fixture();
        psbt.unsigned_tx.output[0].value = Amount::from_sat(98_999);
        assert_eq!(
            validate_regtest_psbt(&psbt, &intent),
            Err(PsbtPolicyError::OutputMismatch)
        );
    }

    #[test]
    fn rejects_fee_above_declared_and_hard_cap() {
        let (psbt, mut intent, _, _, _, _, _) = fixture();
        intent.max_fee_sats = 999;
        assert_eq!(
            validate_regtest_psbt(&psbt, &intent),
            Err(PsbtPolicyError::FeeOutsidePolicy)
        );
        intent.max_fee_sats = MAX_POLICY_FEE_SATS + 1;
        assert_eq!(
            validate_regtest_psbt(&psbt, &intent),
            Err(PsbtPolicyError::InvalidIntent)
        );
    }

    #[test]
    fn rejects_a_mismatched_prevout_commitment() {
        let (mut psbt, intent, _, _, _, _, _) = fixture();
        psbt.inputs[0].witness_utxo.as_mut().expect("UTXO").value = Amount::from_sat(99_999);
        assert_eq!(
            validate_regtest_psbt(&psbt, &intent),
            Err(PsbtPolicyError::PrevoutMismatch)
        );
    }

    #[test]
    fn rejects_non_all_sighash_and_finalized_input_metadata() {
        let (mut psbt, intent, _, _, _, _, _) = fixture();
        psbt.inputs[0].sighash_type =
            Some(PsbtSighashType::from_u32(EcdsaSighashType::None.to_u32()));
        assert_eq!(
            validate_regtest_psbt(&psbt, &intent),
            Err(PsbtPolicyError::UnsupportedSighash)
        );

        let (mut psbt, intent, _, _, _, _, _) = fixture();
        psbt.inputs[0].final_script_witness = Some(Witness::new());
        assert_eq!(
            validate_regtest_psbt(&psbt, &intent),
            Err(PsbtPolicyError::UnsupportedPsbtField)
        );
    }

    #[test]
    fn rejects_unlisted_signer_and_invalid_signature() {
        let (psbt, intent, _, _, maker_key, _, _) = fixture();
        let third_secret = SecretKey::from_slice(&[3u8; 32]).expect("fixture key");
        let unlisted = sign_copy(&psbt, &third_secret, None);
        assert_eq!(
            validate_regtest_psbt(&unlisted, &intent),
            Err(PsbtPolicyError::UnexpectedSigner)
        );

        let forged = sign_copy(&psbt, &third_secret, Some(maker_key));
        assert_eq!(
            validate_regtest_psbt(&forged, &intent),
            Err(PsbtPolicyError::InvalidSignature)
        );
    }

    #[test]
    fn rejects_duplicate_intent_inputs_and_unsupported_headers() {
        let (psbt, mut intent, _, _, _, _, _) = fixture();
        intent.inputs.push(intent.inputs[0].clone());
        assert_eq!(
            validate_regtest_psbt(&psbt, &intent),
            Err(PsbtPolicyError::DuplicateInput)
        );

        let (mut psbt, intent, _, _, _, _, _) = fixture();
        psbt.unsigned_tx.lock_time = LockTime::from_consensus(1);
        assert_eq!(
            validate_regtest_psbt(&psbt, &intent),
            Err(PsbtPolicyError::UnsupportedTransactionHeader)
        );
    }
}
