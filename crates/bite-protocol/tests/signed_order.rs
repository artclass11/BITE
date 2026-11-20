use bite_protocol::{
    verify_signed_btc_usd_order_event, ProtocolError, ValidationPolicy, MAX_EVENT_BYTES,
};
use nostr::prelude::*;
use serde_json::{json, Value};

const NOW: u64 = 1_800_000_000;

fn valid_order(pubkey: &str, now: u64) -> Value {
    json!({
        "version": 1,
        "market": "BTC-USD",
        "orderId": "a".repeat(64),
        "makerPubkey": pubkey,
        "side": "BUY",
        "priceUsdMicrosPerBtc": "100000000000",
        "sizeSats": "1000",
        "createdAt": now,
        "expiresAt": now + 600
    })
}

fn sign_content(content: String, created_at: u64) -> String {
    let keys = Keys::generate();
    EventBuilder::new(Kind::TextNote, content)
        .custom_created_at(Timestamp::from_secs(created_at))
        .finalize(&keys)
        .expect("sign event")
        .try_as_json()
        .expect("serialize signed event")
}

fn signed_order(mut order: Value, event_created_at: u64) -> String {
    let keys = Keys::generate();
    order["makerPubkey"] = json!(keys.public_key().to_hex());
    let content = serde_json::to_string(&order).expect("serialize order");
    EventBuilder::new(Kind::TextNote, content)
        .custom_created_at(Timestamp::from_secs(event_created_at))
        .finalize(&keys)
        .expect("sign event")
        .try_as_json()
        .expect("serialize signed event")
}

fn policy() -> ValidationPolicy {
    ValidationPolicy::at(NOW)
}

#[test]
fn valid_signed_order_verifies_event_and_binds_author_and_timestamp() {
    let event = signed_order(valid_order("", NOW), NOW);
    let verified = verify_signed_btc_usd_order_event(&event, policy()).expect("valid order");
    assert_eq!(verified.order.market, "BTC-USD");
    assert_eq!(verified.order.side, bite_protocol::OrderSide::Buy);
    assert_eq!(verified.order.size_sats, "1000");
    assert_eq!(verified.order.created_at, NOW);
    assert_eq!(verified.maker_pubkey, verified.order.maker_pubkey);
    assert_eq!(verified.event_id.len(), 64);
}

#[test]
fn changed_event_content_fails_nostr_id_or_signature_verification() {
    let event = signed_order(valid_order("", NOW), NOW);
    let mut value: Value = serde_json::from_str(&event).expect("event json");
    value["content"] = json!("{}");
    let tampered = serde_json::to_string(&value).expect("tampered event json");
    assert_eq!(
        verify_signed_btc_usd_order_event(&tampered, policy()),
        Err(ProtocolError::InvalidEventAuthenticity)
    );
}

#[test]
fn signed_event_author_must_match_maker_pubkey_in_order() {
    let keys = Keys::generate();
    let wrong_maker = "c".repeat(64);
    let mut order = valid_order("", NOW);
    order["makerPubkey"] = json!(wrong_maker);
    let content = serde_json::to_string(&order).expect("serialize order");
    let event = EventBuilder::new(Kind::TextNote, content)
        .custom_created_at(Timestamp::from_secs(NOW))
        .finalize(&keys)
        .expect("sign event")
        .try_as_json()
        .expect("serialize event");
    assert_eq!(
        verify_signed_btc_usd_order_event(&event, policy()),
        Err(ProtocolError::EventMakerMismatch)
    );
}

#[test]
fn event_and_payload_timestamps_must_match() {
    let event = signed_order(valid_order("", NOW), NOW + 1);
    assert_eq!(
        verify_signed_btc_usd_order_event(&event, policy()),
        Err(ProtocolError::EventTimestampMismatch)
    );
}

#[test]
fn duplicate_and_unknown_order_fields_are_rejected() {
    let mut order = valid_order("", NOW);
    let duplicate = serde_json::to_string(&order).expect("serialize").replacen(
        "\"version\":1",
        "\"version\":1,\"version\":1",
        1,
    );
    let duplicate_event = sign_content(duplicate, NOW);
    assert_eq!(
        verify_signed_btc_usd_order_event(&duplicate_event, policy()),
        Err(ProtocolError::InvalidOrderJson)
    );

    order["unexpected"] = json!("not allowed");
    let unknown_event = sign_content(serde_json::to_string(&order).unwrap(), NOW);
    assert_eq!(
        verify_signed_btc_usd_order_event(&unknown_event, policy()),
        Err(ProtocolError::InvalidOrderJson)
    );
}

#[test]
fn invalid_order_side_and_non_canonical_amounts_fail_closed() {
    let mut order = valid_order("", NOW);
    order["side"] = json!("SHORT");
    let bad_side = signed_order(order, NOW);
    assert_eq!(
        verify_signed_btc_usd_order_event(&bad_side, policy()),
        Err(ProtocolError::InvalidOrderJson)
    );

    let mut order = valid_order("", NOW);
    order["sizeSats"] = json!("01");
    let bad_amount = signed_order(order, NOW);
    assert_eq!(
        verify_signed_btc_usd_order_event(&bad_amount, policy()),
        Err(ProtocolError::NonCanonicalInteger)
    );
}

#[test]
fn expired_and_overlong_orders_are_rejected() {
    let mut order = valid_order("", NOW);
    order["expiresAt"] = json!(NOW);
    let expired = signed_order(order, NOW);
    assert_eq!(
        verify_signed_btc_usd_order_event(&expired, policy()),
        Err(ProtocolError::InvalidExpiry)
    );

    let mut order = valid_order("", NOW);
    order["expiresAt"] = json!(NOW + 604_801);
    let too_long = signed_order(order, NOW);
    assert_eq!(
        verify_signed_btc_usd_order_event(&too_long, policy()),
        Err(ProtocolError::ExpiryTooFar)
    );
}

#[test]
fn oversized_event_is_rejected_before_parsing() {
    let oversized = " ".repeat(MAX_EVENT_BYTES + 1);
    assert_eq!(
        verify_signed_btc_usd_order_event(&oversized, policy()),
        Err(ProtocolError::EventTooLarge)
    );
}

#[test]
fn top_level_unknown_fields_and_duplicate_fields_are_rejected() {
    let event = signed_order(valid_order("", NOW), NOW);
    let mut value: Value = serde_json::from_str(&event).unwrap();
    value["extra"] = json!("not a NIP-01 field");
    let with_unknown = serde_json::to_string(&value).unwrap();
    assert_eq!(
        verify_signed_btc_usd_order_event(&with_unknown, policy()),
        Err(ProtocolError::InvalidEventJson)
    );

    let duplicate = event.replacen("\"id\":", "\"id\":\"00\",\"id\":", 1);
    assert_eq!(
        verify_signed_btc_usd_order_event(&duplicate, policy()),
        Err(ProtocolError::InvalidEventJson)
    );
}

#[test]
fn policy_requires_a_non_zero_maximum_ttl() {
    let event = signed_order(valid_order("", NOW), NOW);
    let mut invalid_policy = policy();
    invalid_policy.max_ttl_seconds = 0;
    assert_eq!(
        verify_signed_btc_usd_order_event(&event, invalid_policy),
        Err(ProtocolError::InvalidPolicy)
    );
}
