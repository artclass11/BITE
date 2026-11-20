//! BITE's offline BTC/USD order-verification core.
//!
//! This crate verifies a NIP-01 Nostr event ID/signature and binds the signed
//! event author and timestamp to a strict BTC/USD order payload. It does not
//! connect to relays, enforce global replay/uniqueness, match orders, verify
//! Bitcoin ownership, parse PSBTs, observe chain state, or settle USD/BTC.

use nostr::event::Event;
use serde::Deserialize;
use std::collections::HashMap;
use std::fmt;

pub const MARKET_BTC_USD: &str = "BTC-USD";
pub const SATOSHIS_PER_BTC: u128 = 100_000_000;
pub const MAX_BITCOIN_SUPPLY_SATS: u128 = 2_100_000_000_000_000;
pub const MAX_PRICE_MICRO_USD_PER_BTC: u128 = 999_999_999_999_999_999;
pub const DEFAULT_MAX_ORDER_TTL_SECONDS: u64 = 7 * 24 * 60 * 60;
pub const DEFAULT_FUTURE_CLOCK_SKEW_SECONDS: u64 = 30;
pub const MAX_EVENT_BYTES: usize = 16 * 1024;
pub const MAX_ORDER_CONTENT_BYTES: usize = 8 * 1024;
pub const MAX_EVENT_TAGS: usize = 128;
pub const MAX_EVENT_TAG_ELEMENTS: usize = 512;
pub const MAX_EVENT_TAG_VALUE_BYTES: usize = 256;
pub const MAX_REPLAY_GUARD_ENTRIES: usize = 100_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolError {
    EventTooLarge,
    InvalidEventJson,
    InvalidEventAuthenticity,
    EventTooManyTags,
    EventTagsTooLarge,
    OrderContentTooLarge,
    InvalidOrderJson,
    UnsupportedVersion,
    UnsupportedMarket,
    InvalidOrderId,
    InvalidMakerPubkey,
    InvalidSide,
    NonCanonicalInteger,
    OutOfRange,
    InvalidPolicy,
    InvalidExpiry,
    ExpiryTooFar,
    CreatedInFuture,
    OrderExpired,
    EventMakerMismatch,
    EventTimestampMismatch,
    QuoteOverflow,
    InvalidReplayGuardCapacity,
    DuplicateEvent,
    DuplicateOrder,
    ReplayGuardAtCapacity,
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for ProtocolError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct ValidationPolicy {
    pub now_seconds: u64,
    pub max_ttl_seconds: u64,
    pub allowed_future_clock_skew_seconds: u64,
}

impl ValidationPolicy {
    pub const fn at(now_seconds: u64) -> Self {
        Self {
            now_seconds,
            max_ttl_seconds: DEFAULT_MAX_ORDER_TTL_SECONDS,
            allowed_future_clock_skew_seconds: DEFAULT_FUTURE_CLOCK_SKEW_SECONDS,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BtcUsdOrder {
    pub version: u64,
    pub market: String,
    #[serde(rename = "orderId")]
    pub order_id: String,
    #[serde(rename = "makerPubkey")]
    pub maker_pubkey: String,
    pub side: OrderSide,
    #[serde(rename = "priceUsdMicrosPerBtc")]
    pub price_usd_micros_per_btc: String,
    #[serde(rename = "sizeSats")]
    pub size_sats: String,
    #[serde(rename = "createdAt")]
    pub created_at: u64,
    #[serde(rename = "expiresAt")]
    pub expires_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedBtcUsdOrder {
    order: BtcUsdOrder,
    event_id: String,
    maker_pubkey: String,
}

impl VerifiedBtcUsdOrder {
    pub fn order(&self) -> &BtcUsdOrder {
        &self.order
    }

    pub fn event_id(&self) -> &str {
        &self.event_id
    }

    pub fn maker_pubkey(&self) -> &str {
        &self.maker_pubkey
    }
}

/// Bounded, in-process duplicate suppression for a single client order-ingest
/// loop. Feed every relay delivery through the same guard before displaying or
/// matching an order. This is not durable or shared between devices/processes:
/// a production client must persist the same keys atomically across restarts.
#[derive(Debug)]
pub struct ReplayGuard {
    max_entries: usize,
    seen_event_ids: HashMap<String, u64>,
    seen_orders: HashMap<(String, String), u64>,
}

impl ReplayGuard {
    /// Create a guard with a fixed maximum number of simultaneously live
    /// maker/order-ID pairs. It never evicts unexpired entries to make room.
    pub fn new(max_entries: usize) -> Result<Self, ProtocolError> {
        if max_entries == 0 || max_entries > MAX_REPLAY_GUARD_ENTRIES {
            return Err(ProtocolError::InvalidReplayGuardCapacity);
        }
        Ok(Self {
            max_entries,
            seen_event_ids: HashMap::with_capacity(max_entries),
            seen_orders: HashMap::with_capacity(max_entries),
        })
    }

    /// Accept one cryptographically verified order, or reject it as a replay.
    /// Callers must serialize access to one guard for all relay connections.
    pub fn accept(
        &mut self,
        verified: &VerifiedBtcUsdOrder,
        now_seconds: u64,
    ) -> Result<(), ProtocolError> {
        self.prune_expired(now_seconds);

        if verified.order.expires_at <= now_seconds {
            return Err(ProtocolError::OrderExpired);
        }
        if self.seen_event_ids.contains_key(&verified.event_id) {
            return Err(ProtocolError::DuplicateEvent);
        }

        let order_key = (
            verified.maker_pubkey.clone(),
            verified.order.order_id.clone(),
        );
        if self.seen_orders.contains_key(&order_key) {
            return Err(ProtocolError::DuplicateOrder);
        }
        if self.seen_orders.len() >= self.max_entries {
            return Err(ProtocolError::ReplayGuardAtCapacity);
        }

        let expires_at = verified.order.expires_at;
        self.seen_event_ids
            .insert(verified.event_id.clone(), expires_at);
        self.seen_orders.insert(order_key, expires_at);
        Ok(())
    }

    /// Remove only expired entries. Live orders are never silently evicted.
    pub fn prune_expired(&mut self, now_seconds: u64) {
        self.seen_event_ids
            .retain(|_, expires_at| *expires_at > now_seconds);
        self.seen_orders
            .retain(|_, expires_at| *expires_at > now_seconds);
    }

    pub fn len(&self) -> usize {
        self.seen_orders.len()
    }

    pub fn is_empty(&self) -> bool {
        self.seen_orders.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuoteNotional {
    /// Exact numerator in micro-USD. Do not round this value for settlement.
    pub numerator: u128,
    /// Denominator is satoshis per BTC (100,000,000).
    pub denominator: u128,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictNostrEventEnvelope {
    id: String,
    pubkey: String,
    created_at: u64,
    kind: u64,
    tags: Vec<Vec<String>>,
    content: String,
    sig: String,
}

/// Verify the Nostr event ID and Schnorr signature, then validate and bind its
/// order payload to the event author and timestamp.
///
/// The accepted event kind and tag schema are intentionally not fixed here:
/// BITE has not yet approved a dedicated order-event kind or tag contract.
/// The generic Nostr event authenticity check is still mandatory.
pub fn verify_signed_btc_usd_order_event(
    raw_event_json: &str,
    policy: ValidationPolicy,
) -> Result<VerifiedBtcUsdOrder, ProtocolError> {
    if raw_event_json.len() > MAX_EVENT_BYTES {
        return Err(ProtocolError::EventTooLarge);
    }

    // Parse once into a strict shape first. This rejects unknown and duplicate
    // event fields before the maintained Nostr parser handles cryptographic data.
    let envelope: StrictNostrEventEnvelope =
        serde_json::from_str(raw_event_json).map_err(|_| ProtocolError::InvalidEventJson)?;

    if envelope.tags.len() > MAX_EVENT_TAGS {
        return Err(ProtocolError::EventTooManyTags);
    }

    let mut tag_elements = 0usize;
    for tag in &envelope.tags {
        tag_elements = tag_elements
            .checked_add(tag.len())
            .ok_or(ProtocolError::EventTagsTooLarge)?;
        if tag
            .iter()
            .any(|value| value.len() > MAX_EVENT_TAG_VALUE_BYTES)
        {
            return Err(ProtocolError::EventTagsTooLarge);
        }
    }
    if tag_elements > MAX_EVENT_TAG_ELEMENTS {
        return Err(ProtocolError::EventTagsTooLarge);
    }

    if envelope.content.len() > MAX_ORDER_CONTENT_BYTES {
        return Err(ProtocolError::OrderContentTooLarge);
    }

    // Ensure the strict pre-parse and library parse agree on the public fields.
    // The cryptographic library validates the NIP-01 event ID and BIP-340 signature.
    let event = Event::from_json(raw_event_json).map_err(|_| ProtocolError::InvalidEventJson)?;
    if event.id.to_hex() != envelope.id
        || event.pubkey.to_hex() != envelope.pubkey
        || event.created_at.as_secs() != envelope.created_at
        || event.kind.as_u16() as u64 != envelope.kind
        || event.content != envelope.content
        || event.sig.to_string() != envelope.sig
    {
        return Err(ProtocolError::InvalidEventJson);
    }
    event
        .verify()
        .map_err(|_| ProtocolError::InvalidEventAuthenticity)?;

    let order: BtcUsdOrder =
        serde_json::from_str(&event.content).map_err(|_| ProtocolError::InvalidOrderJson)?;
    validate_order(&order, policy)?;

    let event_maker = event.pubkey.to_hex();
    if order.maker_pubkey != event_maker {
        return Err(ProtocolError::EventMakerMismatch);
    }
    if order.created_at != event.created_at.as_secs() {
        return Err(ProtocolError::EventTimestampMismatch);
    }

    Ok(VerifiedBtcUsdOrder {
        order,
        event_id: event.id.to_hex(),
        maker_pubkey: event_maker,
    })
}

pub fn validate_order(order: &BtcUsdOrder, policy: ValidationPolicy) -> Result<(), ProtocolError> {
    if policy.max_ttl_seconds == 0 {
        return Err(ProtocolError::InvalidPolicy);
    }

    if order.version != 1 {
        return Err(ProtocolError::UnsupportedVersion);
    }
    if order.market != MARKET_BTC_USD {
        return Err(ProtocolError::UnsupportedMarket);
    }
    if !is_lower_hex_32(&order.order_id) {
        return Err(ProtocolError::InvalidOrderId);
    }
    if !is_lower_hex_32(&order.maker_pubkey) {
        return Err(ProtocolError::InvalidMakerPubkey);
    }

    parse_positive_canonical_u128(&order.price_usd_micros_per_btc, MAX_PRICE_MICRO_USD_PER_BTC)?;
    parse_positive_canonical_u128(&order.size_sats, MAX_BITCOIN_SUPPLY_SATS)?;

    if order.expires_at <= order.created_at {
        return Err(ProtocolError::InvalidExpiry);
    }
    if order.expires_at - order.created_at > policy.max_ttl_seconds {
        return Err(ProtocolError::ExpiryTooFar);
    }
    if order.created_at
        > policy
            .now_seconds
            .saturating_add(policy.allowed_future_clock_skew_seconds)
    {
        return Err(ProtocolError::CreatedInFuture);
    }
    if order.expires_at <= policy.now_seconds {
        return Err(ProtocolError::OrderExpired);
    }

    Ok(())
}

pub fn quote_notional(order: &BtcUsdOrder) -> Result<QuoteNotional, ProtocolError> {
    let price = parse_positive_canonical_u128(
        &order.price_usd_micros_per_btc,
        MAX_PRICE_MICRO_USD_PER_BTC,
    )?;
    let size = parse_positive_canonical_u128(&order.size_sats, MAX_BITCOIN_SUPPLY_SATS)?;
    let numerator = price
        .checked_mul(size)
        .ok_or(ProtocolError::QuoteOverflow)?;
    Ok(QuoteNotional {
        numerator,
        denominator: SATOSHIS_PER_BTC,
    })
}

fn parse_positive_canonical_u128(value: &str, max: u128) -> Result<u128, ProtocolError> {
    if value.is_empty()
        || !value.bytes().all(|byte| byte.is_ascii_digit())
        || (value.len() > 1 && value.starts_with('0'))
    {
        return Err(ProtocolError::NonCanonicalInteger);
    }

    // Bound decimal length before parsing untrusted strings into big integers.
    if value.len() > max.to_string().len() {
        return Err(ProtocolError::OutOfRange);
    }

    let parsed = value
        .parse::<u128>()
        .map_err(|_| ProtocolError::OutOfRange)?;
    if parsed == 0 || parsed > max {
        return Err(ProtocolError::OutOfRange);
    }
    Ok(parsed)
}

fn is_lower_hex_32(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn canonical_integer_parser_rejects_non_canonical_forms_and_limits_size() {
        assert_eq!(
            parse_positive_canonical_u128("01", 100),
            Err(ProtocolError::NonCanonicalInteger)
        );
        assert_eq!(
            parse_positive_canonical_u128("+1", 100),
            Err(ProtocolError::NonCanonicalInteger)
        );
        assert_eq!(
            parse_positive_canonical_u128("1.0", 100),
            Err(ProtocolError::NonCanonicalInteger)
        );
        assert_eq!(
            parse_positive_canonical_u128("9".repeat(1000).as_str(), 100),
            Err(ProtocolError::OutOfRange)
        );
        assert_eq!(
            parse_positive_canonical_u128("101", 100),
            Err(ProtocolError::OutOfRange)
        );
        assert_eq!(parse_positive_canonical_u128("100", 100), Ok(100));
    }

    #[test]
    fn quote_notional_uses_exact_integer_rational_arithmetic() {
        let order = BtcUsdOrder {
            version: 1,
            market: MARKET_BTC_USD.to_owned(),
            order_id: "a".repeat(64),
            maker_pubkey: "b".repeat(64),
            side: OrderSide::Buy,
            price_usd_micros_per_btc: "100000000000".to_owned(),
            size_sats: "1000".to_owned(),
            created_at: 100,
            expires_at: 200,
        };
        assert_eq!(
            quote_notional(&order),
            Ok(QuoteNotional {
                numerator: 100_000_000_000_000,
                denominator: 100_000_000,
            })
        );
    }
}
