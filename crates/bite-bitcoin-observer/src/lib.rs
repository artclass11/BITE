//! Read-only Bitcoin Core observer for BITE's offline/regtest stage.
//!
//! This crate deliberately exposes only chain, block and transaction reads.
//! It never creates wallets, signs, broadcasts or mutates node state.
//! Connections are restricted to an HTTP loopback endpoint and use Core's
//! cookie-file authentication. The active chain is checked on every query.

use corepc_client::bitcoin::{BlockHash, Txid};
use corepc_client::{Auth, Client};
use serde::Deserialize;
use serde_json::json;
use std::collections::VecDeque;
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

pub const DEFAULT_MAX_REORG_DEPTH: u64 = 144;
pub const MAX_REORG_DEPTH: u64 = 2_016;

/// Configuration for a localhost-only Bitcoin Core RPC connection.
#[derive(Debug, Clone)]
pub struct ObserverConfig {
    rpc_url: String,
    cookie_file: PathBuf,
    max_reorg_depth: u64,
}

impl ObserverConfig {
    pub fn new(
        rpc_url: impl Into<String>,
        cookie_file: impl Into<PathBuf>,
        max_reorg_depth: u64,
    ) -> Result<Self, ObserverError> {
        let rpc_url = rpc_url.into();
        validate_loopback_rpc_url(&rpc_url)?;
        if max_reorg_depth == 0 || max_reorg_depth > MAX_REORG_DEPTH {
            return Err(ObserverError::InvalidReorgDepth);
        }
        Ok(Self {
            rpc_url,
            cookie_file: cookie_file.into(),
            max_reorg_depth,
        })
    }

    pub fn max_reorg_depth(&self) -> u64 {
        self.max_reorg_depth
    }
}

/// Fail-closed observer errors. Raw authentication material is never included.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObserverError {
    InvalidRpcUrl,
    InvalidReorgDepth,
    MissingCookieFile,
    Rpc(String),
    UnexpectedNetwork(String),
    InvalidRpcResponse,
    HeightOutOfRange,
    InconsistentChainTip,
    BlockNotActive,
    BlockHashMismatch,
    BlockHeightMismatch,
    PreviousBlockMismatch,
    TransactionNotInBlock,
    TransactionIdMismatch,
    TransactionNotActive,
    ChainChangedDuringObservation,
    InvalidChainWindow,
    ReorgBeyondTrackedWindow,
}

impl fmt::Display for ObserverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRpcUrl => write!(f, "RPC URL must be an HTTP loopback endpoint with an explicit port and no embedded credentials"),
            Self::InvalidReorgDepth => write!(f, "reorg tracking depth must be between 1 and 2016 blocks"),
            Self::MissingCookieFile => write!(f, "Bitcoin Core RPC cookie file is missing or unreadable"),
            Self::Rpc(message) => write!(f, "Bitcoin Core RPC error: {message}"),
            Self::UnexpectedNetwork(network) => write!(f, "refusing unexpected Bitcoin network: {network}"),
            Self::InvalidRpcResponse => write!(f, "Bitcoin Core returned an invalid or incomplete RPC response"),
            Self::HeightOutOfRange => write!(f, "block height is outside the supported RPC range"),
            Self::InconsistentChainTip => write!(f, "Bitcoin Core chain tip changed or disagreed during observation"),
            Self::BlockNotActive => write!(f, "requested block is not in the active chain"),
            Self::BlockHashMismatch => write!(f, "block body hash did not match the requested block hash"),
            Self::BlockHeightMismatch => write!(f, "block header height did not match the requested height"),
            Self::PreviousBlockMismatch => write!(f, "block header ancestry did not match its RPC metadata"),
            Self::TransactionNotInBlock => write!(f, "requested transaction is not contained in the specified block"),
            Self::TransactionIdMismatch => write!(f, "Bitcoin Core returned a different transaction ID"),
            Self::TransactionNotActive => write!(f, "transaction is not confirmed in the active chain"),
            Self::ChainChangedDuringObservation => write!(f, "active chain changed while the transaction was being observed; retry the read"),
            Self::InvalidChainWindow => write!(f, "chain window is empty, non-contiguous, or has an inconsistent tip"),
            Self::ReorgBeyondTrackedWindow => write!(f, "no common ancestor exists within the configured reorg tracking window; resynchronization is required"),
        }
    }
}

impl std::error::Error for ObserverError {}

/// A consistent view of the active chain tip returned by getblockchaininfo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainTip {
    pub height: u64,
    pub hash: BlockHash,
    pub initial_block_download: bool,
}

/// Block identity at a particular height.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockRef {
    pub height: u64,
    pub hash: BlockHash,
}

/// A bounded contiguous window of canonical active-chain block identities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainWindow {
    pub tip: ChainTip,
    pub blocks: Vec<BlockRef>,
}

impl ChainWindow {
    pub fn validate(&self) -> Result<(), ObserverError> {
        let Some(last) = self.blocks.last() else {
            return Err(ObserverError::InvalidChainWindow);
        };
        if last.height != self.tip.height || last.hash != self.tip.hash {
            return Err(ObserverError::InvalidChainWindow);
        }
        for pair in self.blocks.windows(2) {
            if pair[0].height.checked_add(1) != Some(pair[1].height) {
                return Err(ObserverError::InvalidChainWindow);
            }
        }
        Ok(())
    }
}

/// Result of comparing one canonical chain window with the previous window.
/// A reorg explicitly identifies disconnected and replacement blocks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainUpdate {
    Initialized {
        tip: ChainTip,
    },
    Unchanged {
        tip: ChainTip,
    },
    Extended {
        common_ancestor: BlockRef,
        added: Vec<BlockRef>,
        tip: ChainTip,
    },
    Reorganized {
        common_ancestor: BlockRef,
        removed: Vec<BlockRef>,
        added: Vec<BlockRef>,
        tip: ChainTip,
    },
}

/// Small in-memory canonical-chain tracker. Persisting this cursor is a future
/// responsibility; after restart, initialize from a fresh bounded window.
#[derive(Debug, Clone)]
pub struct ChainTracker {
    max_reorg_depth: u64,
    known: VecDeque<BlockRef>,
}

impl ChainTracker {
    pub fn new(max_reorg_depth: u64) -> Result<Self, ObserverError> {
        if max_reorg_depth == 0 || max_reorg_depth > MAX_REORG_DEPTH {
            return Err(ObserverError::InvalidReorgDepth);
        }
        Ok(Self {
            max_reorg_depth,
            known: VecDeque::new(),
        })
    }

    pub fn apply(&mut self, window: ChainWindow) -> Result<ChainUpdate, ObserverError> {
        window.validate()?;
        if window.blocks.len() as u64 > self.max_reorg_depth.saturating_add(1) {
            return Err(ObserverError::InvalidChainWindow);
        }

        if self.known.is_empty() {
            self.known = window.blocks.iter().copied().collect();
            return Ok(ChainUpdate::Initialized { tip: window.tip });
        }

        let common_ancestor = window
            .blocks
            .iter()
            .rev()
            .find(|candidate| self.known.iter().any(|old| old == *candidate))
            .copied()
            .ok_or(ObserverError::ReorgBeyondTrackedWindow)?;

        let removed: Vec<BlockRef> = self
            .known
            .iter()
            .copied()
            .filter(|block| block.height > common_ancestor.height)
            .collect();
        let added: Vec<BlockRef> = window
            .blocks
            .iter()
            .copied()
            .filter(|block| block.height > common_ancestor.height)
            .collect();

        self.known = window.blocks.iter().copied().collect();

        if removed.is_empty() && added.is_empty() {
            Ok(ChainUpdate::Unchanged { tip: window.tip })
        } else if removed.is_empty() {
            Ok(ChainUpdate::Extended {
                common_ancestor,
                added,
                tip: window.tip,
            })
        } else {
            Ok(ChainUpdate::Reorganized {
                common_ancestor,
                removed,
                added,
                tip: window.tip,
            })
        }
    }

    pub fn tracked_blocks(&self) -> Vec<BlockRef> {
        self.known.iter().copied().collect()
    }
}

/// Fully checked read-only details for one active-chain block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedBlock {
    pub height: u64,
    pub hash: BlockHash,
    pub previous_block_hash: Option<BlockHash>,
    pub confirmations: u64,
    pub transaction_ids: Vec<Txid>,
    pub median_time: Option<u64>,
}

/// Transaction inclusion and active-chain confirmation status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionObservation {
    pub txid: Txid,
    pub block_hash: BlockHash,
    pub block_height: u64,
    pub confirmations: u64,
}

pub struct BitcoinCoreObserver {
    client: Client,
    max_reorg_depth: u64,
}

impl BitcoinCoreObserver {
    /// Connect using Bitcoin Core's cookie file. The endpoint must be loopback.
    pub async fn connect(config: ObserverConfig) -> Result<Self, ObserverError> {
        if !config.cookie_file.is_file() {
            return Err(ObserverError::MissingCookieFile);
        }
        let client = Client::new_with_auth(
            &config.rpc_url,
            Auth::CookieFile(config.cookie_file.clone()),
        )
        .map_err(|error| ObserverError::Rpc(error.to_string()))?;
        let observer = Self {
            client,
            max_reorg_depth: config.max_reorg_depth,
        };
        // Refuse to proceed unless the node explicitly identifies as regtest.
        observer.chain_tip().await?;
        Ok(observer)
    }

    pub fn max_reorg_depth(&self) -> u64 {
        self.max_reorg_depth
    }

    /// Read the current chain tip and cross-check its hash against getblockhash.
    pub async fn chain_tip(&self) -> Result<ChainTip, ObserverError> {
        let info: RpcBlockchainInfo = self
            .client
            .call("getblockchaininfo", &[])
            .await
            .map_err(|error| ObserverError::Rpc(error.to_string()))?;
        if info.chain != "regtest" {
            return Err(ObserverError::UnexpectedNetwork(info.chain));
        }
        let height = u64::try_from(info.blocks).map_err(|_| ObserverError::InvalidRpcResponse)?;
        let reported_hash = parse_block_hash(&info.bestblockhash)?;
        let rpc_height = u32::try_from(height).map_err(|_| ObserverError::HeightOutOfRange)?;
        let canonical_hash = self
            .client
            .get_block_hash(rpc_height)
            .await
            .map_err(|error| ObserverError::Rpc(error.to_string()))?;
        if canonical_hash != reported_hash {
            return Err(ObserverError::InconsistentChainTip);
        }
        Ok(ChainTip {
            height,
            hash: reported_hash,
            initial_block_download: info.initialblockdownload,
        })
    }

    /// Fetch a bounded window of active-chain hashes for reorg tracking.
    pub async fn chain_window(&self) -> Result<ChainWindow, ObserverError> {
        let tip = self.chain_tip().await?;
        let first_height = tip.height.saturating_sub(self.max_reorg_depth);
        let mut blocks = Vec::with_capacity((tip.height - first_height + 1) as usize);
        for height in first_height..=tip.height {
            let rpc_height = u32::try_from(height).map_err(|_| ObserverError::HeightOutOfRange)?;
            let hash = self
                .client
                .get_block_hash(rpc_height)
                .await
                .map_err(|error| ObserverError::Rpc(error.to_string()))?;
            blocks.push(BlockRef { height, hash });
        }
        let window = ChainWindow { tip, blocks };
        window.validate()?;
        Ok(window)
    }

    /// Poll the active chain and report extensions, unchanged tips or reorgs.
    pub async fn sync_chain(
        &self,
        tracker: &mut ChainTracker,
    ) -> Result<ChainUpdate, ObserverError> {
        tracker.apply(self.chain_window().await?)
    }

    /// Fetch a block by height and check its hash, height, ancestry and txids.
    pub async fn observe_block(&self, height: u64) -> Result<ObservedBlock, ObserverError> {
        let tip = self.chain_tip().await?;
        if height > tip.height {
            return Err(ObserverError::BlockNotActive);
        }
        let rpc_height = u32::try_from(height).map_err(|_| ObserverError::HeightOutOfRange)?;
        let expected_hash = self
            .client
            .get_block_hash(rpc_height)
            .await
            .map_err(|error| ObserverError::Rpc(error.to_string()))?;

        let header: RpcBlockHeaderVerbose = self
            .client
            .call(
                "getblockheader",
                &[json!(expected_hash.to_string()), json!(true)],
            )
            .await
            .map_err(|error| ObserverError::Rpc(error.to_string()))?;
        if header.confirmations <= 0 {
            return Err(ObserverError::BlockNotActive);
        }
        let header_height =
            u64::try_from(header.height).map_err(|_| ObserverError::InvalidRpcResponse)?;
        if header_height != height {
            return Err(ObserverError::BlockHeightMismatch);
        }
        let header_hash = parse_block_hash(&header.hash)?;
        if header_hash != expected_hash {
            return Err(ObserverError::BlockHashMismatch);
        }

        let block = self
            .client
            .get_block(&expected_hash)
            .await
            .map_err(|error| ObserverError::Rpc(error.to_string()))?;
        if block.header.block_hash() != expected_hash {
            return Err(ObserverError::BlockHashMismatch);
        }
        let previous_block_hash = if height == 0 {
            None
        } else {
            Some(block.header.prev_blockhash)
        };
        let rpc_previous = header
            .previous_block_hash
            .as_deref()
            .map(parse_block_hash)
            .transpose()?;
        if previous_block_hash != rpc_previous {
            return Err(ObserverError::PreviousBlockMismatch);
        }

        // Ensure the block remained on the active chain while its body was read.
        let after_hash = self
            .client
            .get_block_hash(rpc_height)
            .await
            .map_err(|error| ObserverError::Rpc(error.to_string()))?;
        if after_hash != expected_hash {
            return Err(ObserverError::ChainChangedDuringObservation);
        }

        let transaction_ids = block.txdata.iter().map(|tx| tx.compute_txid()).collect();
        Ok(ObservedBlock {
            height,
            hash: expected_hash,
            previous_block_hash,
            confirmations: u64::try_from(header.confirmations)
                .map_err(|_| ObserverError::InvalidRpcResponse)?,
            transaction_ids,
            median_time: header.median_time.and_then(|time| u64::try_from(time).ok()),
        })
    }

    /// Look up a transaction in a specific block. The block-hash argument is
    /// mandatory, so this works without txindex and cannot mistake mempool data
    /// for a confirmed transaction.
    pub async fn observe_transaction(
        &self,
        txid: Txid,
        containing_block: BlockHash,
    ) -> Result<TransactionObservation, ObserverError> {
        let verbose: RpcRawTransactionVerbose = self
            .client
            .call(
                "getrawtransaction",
                &[
                    json!(txid.to_string()),
                    json!(true),
                    json!(containing_block.to_string()),
                ],
            )
            .await
            .map_err(|error| ObserverError::Rpc(error.to_string()))?;

        if parse_txid(&verbose.txid)? != txid {
            return Err(ObserverError::TransactionIdMismatch);
        }
        let returned_block = verbose
            .block_hash
            .as_deref()
            .map(parse_block_hash)
            .transpose()?
            .ok_or(ObserverError::TransactionNotActive)?;
        if returned_block != containing_block || verbose.in_active_chain == Some(false) {
            return Err(ObserverError::TransactionNotActive);
        }
        if verbose.confirmations.unwrap_or(0) <= 0 {
            return Err(ObserverError::TransactionNotActive);
        }

        let header: RpcBlockHeaderVerbose = self
            .client
            .call(
                "getblockheader",
                &[json!(containing_block.to_string()), json!(true)],
            )
            .await
            .map_err(|error| ObserverError::Rpc(error.to_string()))?;
        if header.confirmations <= 0 {
            return Err(ObserverError::TransactionNotActive);
        }
        let block_height =
            u64::try_from(header.height).map_err(|_| ObserverError::InvalidRpcResponse)?;
        let block = self.observe_block(block_height).await?;
        if block.hash != containing_block || !block.transaction_ids.contains(&txid) {
            return Err(ObserverError::TransactionNotInBlock);
        }

        let tip = self.chain_tip().await?;
        if self
            .client
            .get_block_hash(
                u32::try_from(block_height).map_err(|_| ObserverError::HeightOutOfRange)?,
            )
            .await
            .map_err(|error| ObserverError::Rpc(error.to_string()))?
            != containing_block
            || tip.height < block_height
        {
            return Err(ObserverError::ChainChangedDuringObservation);
        }
        Ok(TransactionObservation {
            txid,
            block_hash: containing_block,
            block_height,
            confirmations: tip.height - block_height + 1,
        })
    }
}

#[derive(Debug, Deserialize)]
struct RpcBlockchainInfo {
    chain: String,
    blocks: i64,
    bestblockhash: String,
    initialblockdownload: bool,
}

#[derive(Debug, Deserialize)]
struct RpcBlockHeaderVerbose {
    hash: String,
    confirmations: i64,
    height: i64,
    #[serde(default, rename = "previousblockhash")]
    previous_block_hash: Option<String>,
    #[serde(default, rename = "mediantime")]
    median_time: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct RpcRawTransactionVerbose {
    txid: String,
    #[serde(default, rename = "blockhash")]
    block_hash: Option<String>,
    #[serde(default)]
    confirmations: Option<i64>,
    #[serde(default, rename = "in_active_chain")]
    in_active_chain: Option<bool>,
}

fn parse_block_hash(value: &str) -> Result<BlockHash, ObserverError> {
    BlockHash::from_str(value).map_err(|_| ObserverError::InvalidRpcResponse)
}

fn parse_txid(value: &str) -> Result<Txid, ObserverError> {
    Txid::from_str(value).map_err(|_| ObserverError::InvalidRpcResponse)
}

fn validate_loopback_rpc_url(url: &str) -> Result<(), ObserverError> {
    let authority = url
        .strip_prefix("http://127.0.0.1:")
        .or_else(|| url.strip_prefix("http://localhost:"))
        .or_else(|| url.strip_prefix("http://[::1]:"))
        .ok_or(ObserverError::InvalidRpcUrl)?;
    if authority.contains('/')
        || authority.contains('@')
        || authority.contains('?')
        || authority.contains('#')
    {
        return Err(ObserverError::InvalidRpcUrl);
    }
    let port = authority
        .parse::<u16>()
        .map_err(|_| ObserverError::InvalidRpcUrl)?;
    if port == 0 {
        return Err(ObserverError::InvalidRpcUrl);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(byte: char) -> BlockHash {
        BlockHash::from_str(&byte.to_string().repeat(64)).expect("valid block hash")
    }

    fn ref_at(height: u64, byte: char) -> BlockRef {
        BlockRef {
            height,
            hash: hash(byte),
        }
    }

    fn window(blocks: Vec<BlockRef>) -> ChainWindow {
        let tip_ref = *blocks.last().expect("window has a tip");
        ChainWindow {
            tip: ChainTip {
                height: tip_ref.height,
                hash: tip_ref.hash,
                initial_block_download: false,
            },
            blocks,
        }
    }

    #[test]
    fn rpc_url_must_be_loopback_without_embedded_credentials() {
        assert!(ObserverConfig::new("http://127.0.0.1:18443", "/tmp/cookie", 12).is_ok());
        assert!(ObserverConfig::new("http://localhost:18443", "/tmp/cookie", 12).is_ok());
        assert!(matches!(
            ObserverConfig::new("http://0.0.0.0:18443", "/tmp/cookie", 12),
            Err(ObserverError::InvalidRpcUrl)
        ));
        assert!(matches!(
            ObserverConfig::new("http://user:pass@127.0.0.1:18443", "/tmp/cookie", 12),
            Err(ObserverError::InvalidRpcUrl)
        ));
    }

    #[test]
    fn tracker_distinguishes_initialization_extension_and_unchanged_tip() {
        let mut tracker = ChainTracker::new(8).unwrap();
        let initial = window(vec![ref_at(0, 'a'), ref_at(1, 'b'), ref_at(2, 'c')]);
        assert!(matches!(
            tracker.apply(initial).unwrap(),
            ChainUpdate::Initialized { .. }
        ));

        let unchanged = window(vec![ref_at(0, 'a'), ref_at(1, 'b'), ref_at(2, 'c')]);
        assert!(matches!(
            tracker.apply(unchanged).unwrap(),
            ChainUpdate::Unchanged { .. }
        ));

        let extended = window(vec![
            ref_at(0, 'a'),
            ref_at(1, 'b'),
            ref_at(2, 'c'),
            ref_at(3, 'd'),
        ]);
        match tracker.apply(extended).unwrap() {
            ChainUpdate::Extended {
                common_ancestor,
                added,
                tip,
            } => {
                assert_eq!(common_ancestor, ref_at(2, 'c'));
                assert_eq!(added, vec![ref_at(3, 'd')]);
                assert_eq!(tip.height, 3);
            }
            other => panic!("expected extension, got {other:?}"),
        }
    }

    #[test]
    fn tracker_reports_removed_and_replacement_blocks_for_a_reorg() {
        let mut tracker = ChainTracker::new(8).unwrap();
        tracker
            .apply(window(vec![
                ref_at(0, 'a'),
                ref_at(1, 'b'),
                ref_at(2, 'c'),
                ref_at(3, 'd'),
            ]))
            .unwrap();

        let update = tracker
            .apply(window(vec![
                ref_at(0, 'a'),
                ref_at(1, 'b'),
                ref_at(2, 'e'),
                ref_at(3, 'f'),
                ref_at(4, 'g'),
            ]))
            .unwrap();
        match update {
            ChainUpdate::Reorganized {
                common_ancestor,
                removed,
                added,
                tip,
            } => {
                assert_eq!(common_ancestor, ref_at(1, 'b'));
                assert_eq!(removed, vec![ref_at(2, 'c'), ref_at(3, 'd')]);
                assert_eq!(added, vec![ref_at(2, 'e'), ref_at(3, 'f'), ref_at(4, 'g')]);
                assert_eq!(tip.height, 4);
            }
            other => panic!("expected reorg, got {other:?}"),
        }
    }

    #[test]
    fn tracker_fails_closed_when_reorg_exceeds_retained_window() {
        let mut tracker = ChainTracker::new(2).unwrap();
        tracker
            .apply(window(vec![
                ref_at(10, 'a'),
                ref_at(11, 'b'),
                ref_at(12, 'c'),
            ]))
            .unwrap();
        let incoming = window(vec![ref_at(11, 'x'), ref_at(12, 'y'), ref_at(13, 'z')]);
        assert_eq!(
            tracker.apply(incoming),
            Err(ObserverError::ReorgBeyondTrackedWindow)
        );
    }

    #[test]
    fn chain_window_rejects_non_contiguous_heights_and_wrong_tip() {
        assert_eq!(
            window(vec![ref_at(1, 'a'), ref_at(3, 'b')]).validate(),
            Err(ObserverError::InvalidChainWindow)
        );
        let mut bad_tip = window(vec![ref_at(1, 'a'), ref_at(2, 'b')]);
        bad_tip.tip.hash = hash('c');
        assert_eq!(bad_tip.validate(), Err(ObserverError::InvalidChainWindow));
    }
}
