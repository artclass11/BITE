//! Durable, local replay protection for verified BTC/USD orders.
//!
//! This SQLite store survives process restarts. Its uniqueness constraints and
//! IMMEDIATE write transactions coordinate separate connections/processes that
//! share the same database file. It is still local to that file: it does not
//! provide network-wide or cross-device consensus.

use crate::{
    ProtocolError, VerifiedBtcUsdOrder, MAX_REPLAY_GUARD_ENTRIES,
};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use std::fmt;
use std::path::Path;
use std::time::Duration;

const SCHEMA_VERSION: i64 = 1;
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// Failure returned by the durable replay store.
#[derive(Debug)]
pub enum ReplayStoreError {
    /// A protocol-level rejection, such as a duplicate or expired order.
    Protocol(ProtocolError),
    /// SQLite could not safely complete the requested operation.
    Storage(rusqlite::Error),
    /// The database was opened with a different capacity than its first setup.
    ConfigurationMismatch,
    /// The database schema is newer or otherwise unsupported by this binary.
    UnsupportedSchemaVersion(i64),
    /// The stored live-entry count could not be represented by this platform.
    InvalidEntryCount,
}

impl fmt::Display for ReplayStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Protocol(error) => write!(f, "replay rejected: {error}"),
            Self::Storage(error) => write!(f, "replay database error: {error}"),
            Self::ConfigurationMismatch => {
                write!(f, "replay database capacity differs from configured capacity")
            }
            Self::UnsupportedSchemaVersion(version) => {
                write!(f, "unsupported replay database schema version: {version}")
            }
            Self::InvalidEntryCount => write!(f, "invalid replay database entry count"),
        }
    }
}

impl std::error::Error for ReplayStoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Protocol(error) => Some(error),
            Self::Storage(error) => Some(error),
            _ => None,
        }
    }
}

impl From<rusqlite::Error> for ReplayStoreError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Storage(error)
    }
}

impl From<ProtocolError> for ReplayStoreError {
    fn from(error: ProtocolError) -> Self {
        Self::Protocol(error)
    }
}

/// SQLite-backed duplicate suppression for one client's order-ingest state.
///
/// Every order must first be returned by
/// `verify_signed_btc_usd_order_event`. The store persists only public order
/// identifiers, maker public keys, event IDs and expiries; it never stores
/// signing keys or payment details.
///
/// Use one database file for all relay feeds handled by a client. SQLite's
/// transaction lock and unique constraints also coordinate separate local
/// processes sharing that file. This does not synchronize different devices,
/// clients, or independent databases. A relay or a local cache is not a
/// globally authoritative order registry.
pub struct DurableReplayStore {
    connection: Connection,
    max_entries: usize,
}

impl fmt::Debug for DurableReplayStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DurableReplayStore")
            .field("max_entries", &self.max_entries)
            .finish_non_exhaustive()
    }
}

impl DurableReplayStore {
    /// Open or create a durable database at `path`.
    ///
    /// The first open fixes the capacity in the database metadata. Later opens
    /// must use the same value so an accidental configuration change cannot
    /// silently increase or decrease the admission limit.
    pub fn open(
        path: impl AsRef<Path>,
        max_entries: usize,
    ) -> Result<Self, ReplayStoreError> {
        validate_capacity(max_entries)?;
        let connection = Connection::open(path)?;
        Self::from_connection(connection, max_entries)
    }

    /// Create an in-memory database. Intended for tests and short-lived tools;
    /// it does not provide persistence across process restarts.
    pub fn open_in_memory(max_entries: usize) -> Result<Self, ReplayStoreError> {
        validate_capacity(max_entries)?;
        let connection = Connection::open_in_memory()?;
        Self::from_connection(connection, max_entries)
    }

    fn from_connection(
        mut connection: Connection,
        max_entries: usize,
    ) -> Result<Self, ReplayStoreError> {
        connection.busy_timeout(BUSY_TIMEOUT)?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.pragma_update(None, "trusted_schema", "OFF")?;

        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS replay_store_meta (
                singleton INTEGER NOT NULL PRIMARY KEY CHECK (singleton = 1),
                schema_version INTEGER NOT NULL,
                max_entries INTEGER NOT NULL CHECK (max_entries BETWEEN 1 AND 100000)
             );

             CREATE TABLE IF NOT EXISTS replay_entries (
                event_id TEXT NOT NULL PRIMARY KEY,
                maker_pubkey TEXT NOT NULL,
                order_id TEXT NOT NULL,
                expires_at INTEGER NOT NULL CHECK (expires_at > 0),
                UNIQUE (maker_pubkey, order_id)
             ) WITHOUT ROWID;

             CREATE INDEX IF NOT EXISTS replay_entries_expiry
                ON replay_entries (expires_at);",
        )?;

        let configured_capacity = i64::try_from(max_entries)
            .map_err(|_| ProtocolError::InvalidReplayGuardCapacity)?;
        let transaction =
            connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let stored: Option<(i64, i64)> = transaction
            .query_row(
                "SELECT schema_version, max_entries
                 FROM replay_store_meta
                 WHERE singleton = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;

        match stored {
            Some((version, _)) if version != SCHEMA_VERSION => {
                return Err(ReplayStoreError::UnsupportedSchemaVersion(version));
            }
            Some((_, capacity)) if capacity != configured_capacity => {
                return Err(ReplayStoreError::ConfigurationMismatch);
            }
            Some(_) => {}
            None => {
                transaction.execute(
                    "INSERT INTO replay_store_meta (singleton, schema_version, max_entries)
                     VALUES (1, ?1, ?2)",
                    params![SCHEMA_VERSION, configured_capacity],
                )?;
            }
        }
        transaction.commit()?;

        Ok(Self {
            connection,
            max_entries,
        })
    }

    /// Atomically admit a cryptographically verified order.
    ///
    /// Expired entries are pruned inside the same IMMEDIATE transaction as the
    /// duplicate checks, capacity check and insertion. Unexpired rows are
    /// never evicted to make room. Any storage failure is returned to the
    /// caller: callers must fail closed and must not display/match the order.
    pub fn accept(
        &mut self,
        verified: &VerifiedBtcUsdOrder,
        now_seconds: u64,
    ) -> Result<(), ReplayStoreError> {
        let now = i64::try_from(now_seconds)
            .map_err(|_| ReplayStoreError::Protocol(ProtocolError::ReplayTimestampOutOfRange))?;
        let expires_at = i64::try_from(verified.order().expires_at)
            .map_err(|_| ReplayStoreError::Protocol(ProtocolError::ReplayTimestampOutOfRange))?;

        if expires_at <= now {
            return Err(ProtocolError::OrderExpired.into());
        }

        let transaction =
            self.connection.transaction_with_behavior(TransactionBehavior::Immediate)?;

        transaction.execute(
            "DELETE FROM replay_entries WHERE expires_at <= ?1",
            [now],
        )?;

        let duplicate_event: bool = transaction.query_row(
            "SELECT EXISTS(
                SELECT 1 FROM replay_entries WHERE event_id = ?1
             )",
            [verified.event_id()],
            |row| row.get(0),
        )?;
        if duplicate_event {
            return Err(ProtocolError::DuplicateEvent.into());
        }

        let duplicate_order: bool = transaction.query_row(
            "SELECT EXISTS(
                SELECT 1 FROM replay_entries
                WHERE maker_pubkey = ?1 AND order_id = ?2
             )",
            params![verified.maker_pubkey(), &verified.order().order_id],
            |row| row.get(0),
        )?;
        if duplicate_order {
            return Err(ProtocolError::DuplicateOrder.into());
        }

        let count: i64 = transaction.query_row(
            "SELECT COUNT(*) FROM replay_entries",
            [],
            |row| row.get(0),
        )?;
        let count = usize::try_from(count).map_err(|_| ReplayStoreError::InvalidEntryCount)?;
        if count >= self.max_entries {
            return Err(ProtocolError::ReplayGuardAtCapacity.into());
        }

        transaction.execute(
            "INSERT INTO replay_entries (event_id, maker_pubkey, order_id, expires_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                verified.event_id(),
                verified.maker_pubkey(),
                &verified.order().order_id,
                expires_at
            ],
        )?;
        transaction.commit()?;
        Ok(())
    }

    /// Remove entries whose expiry is at or before `now_seconds`.
    pub fn prune_expired(&mut self, now_seconds: u64) -> Result<usize, ReplayStoreError> {
        let now = i64::try_from(now_seconds)
            .map_err(|_| ReplayStoreError::Protocol(ProtocolError::ReplayTimestampOutOfRange))?;
        let transaction =
            self.connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let removed = transaction.execute(
            "DELETE FROM replay_entries WHERE expires_at <= ?1",
            [now],
        )?;
        transaction.commit()?;
        Ok(removed)
    }

    /// Return the number of rows currently stored, including entries that
    /// have expired but have not yet been pruned by an admission or prune call.
    pub fn len(&self) -> Result<usize, ReplayStoreError> {
        let count: i64 = self.connection.query_row(
            "SELECT COUNT(*) FROM replay_entries",
            [],
            |row| row.get(0),
        )?;
        usize::try_from(count).map_err(|_| ReplayStoreError::InvalidEntryCount)
    }

    pub fn is_empty(&self) -> Result<bool, ReplayStoreError> {
        Ok(self.len()? == 0)
    }

    pub fn max_entries(&self) -> usize {
        self.max_entries
    }
}

fn validate_capacity(max_entries: usize) -> Result<(), ProtocolError> {
    if max_entries == 0 || max_entries > MAX_REPLAY_GUARD_ENTRIES {
        return Err(ProtocolError::InvalidReplayGuardCapacity);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BtcUsdOrder, OrderSide};
    use std::ffi::OsString;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT_DATABASE_ID: AtomicU64 = AtomicU64::new(0);

    fn temporary_database_path() -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let sequence = NEXT_DATABASE_ID.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "bite-replay-{}-{timestamp}-{sequence}.sqlite",
            std::process::id()
        ))
    }

    fn cleanup_database(path: &Path) {
        let _ = fs::remove_file(path);
        for suffix in ["-wal", "-shm"] {
            let mut sidecar = OsString::from(path.as_os_str());
            sidecar.push(suffix);
            let _ = fs::remove_file(Path::new(&sidecar));
        }
    }

    fn verified(
        event_byte: char,
        maker_byte: char,
        order_byte: char,
        expires_at: u64,
    ) -> VerifiedBtcUsdOrder {
        let maker_pubkey = maker_byte.to_string().repeat(64);
        let order = BtcUsdOrder {
            version: 1,
            market: "BTC-USD".to_owned(),
            order_id: order_byte.to_string().repeat(64),
            maker_pubkey: maker_pubkey.clone(),
            side: OrderSide::Buy,
            price_usd_micros_per_btc: "100000000000".to_owned(),
            size_sats: "1000".to_owned(),
            created_at: 100,
            expires_at,
        };
        VerifiedBtcUsdOrder {
            order,
            event_id: event_byte.to_string().repeat(64),
            maker_pubkey,
        }
    }

    fn expect_protocol_error(
        result: Result<(), ReplayStoreError>,
        expected: ProtocolError,
    ) {
        assert!(matches!(
            result,
            Err(ReplayStoreError::Protocol(actual)) if actual == expected
        ));
    }

    #[test]
    fn durable_store_rejects_replays_after_reopening_the_database() {
        let path = temporary_database_path();
        let first_order = verified('1', 'a', 'b', 300);

        {
            let mut store = DurableReplayStore::open(&path, 8).unwrap();
            assert_eq!(store.accept(&first_order, 150), Ok(()));
            assert_eq!(store.len(), Ok(1));
        }

        {
            let mut reopened = DurableReplayStore::open(&path, 8).unwrap();
            expect_protocol_error(
                reopened.accept(&first_order, 151),
                ProtocolError::DuplicateEvent,
            );

            let same_order_new_event = verified('2', 'a', 'b', 301);
            expect_protocol_error(
                reopened.accept(&same_order_new_event, 151),
                ProtocolError::DuplicateOrder,
            );
            assert_eq!(reopened.len(), Ok(1));
        }

        cleanup_database(&path);
    }

    #[test]
    fn separate_connections_sharing_a_file_observe_the_same_unique_keys() {
        let path = temporary_database_path();
        let mut first = DurableReplayStore::open(&path, 8).unwrap();
        let mut second = DurableReplayStore::open(&path, 8).unwrap();
        let order = verified('1', 'a', 'b', 300);

        assert_eq!(first.accept(&order, 150), Ok(()));
        expect_protocol_error(
            second.accept(&order, 150),
            ProtocolError::DuplicateEvent,
        );
        assert_eq!(second.len(), Ok(1));

        drop(first);
        drop(second);
        cleanup_database(&path);
    }

    #[test]
    fn durable_store_prunes_only_expired_rows_before_capacity_check() {
        let mut store = DurableReplayStore::open_in_memory(1).unwrap();
        let first = verified('1', 'a', 'b', 110);
        let second = verified('2', 'a', 'c', 200);

        assert_eq!(store.accept(&first, 100), Ok(()));
        assert_eq!(store.accept(&second, 110), Ok(()));
        assert_eq!(store.len(), Ok(1));
        assert_eq!(store.prune_expired(199), Ok(0));
        assert_eq!(store.prune_expired(200), Ok(1));
        assert!(store.is_empty().unwrap());
    }

    #[test]
    fn durable_store_fails_closed_at_capacity_without_evicting_live_rows() {
        let mut store = DurableReplayStore::open_in_memory(1).unwrap();
        let first = verified('1', 'a', 'b', 300);
        let second = verified('2', 'a', 'c', 301);

        assert_eq!(store.accept(&first, 150), Ok(()));
        expect_protocol_error(
            store.accept(&second, 151),
            ProtocolError::ReplayGuardAtCapacity,
        );
        assert_eq!(store.len(), Ok(1));
        expect_protocol_error(
            store.accept(&first, 151),
            ProtocolError::DuplicateEvent,
        );
    }

    #[test]
    fn durable_store_rejects_capacity_changes_on_reopen() {
        let path = temporary_database_path();
        let store = DurableReplayStore::open(&path, 4).unwrap();
        drop(store);

        assert!(matches!(
            DurableReplayStore::open(&path, 5),
            Err(ReplayStoreError::ConfigurationMismatch)
        ));

        cleanup_database(&path);
    }

    #[test]
    fn durable_store_rejects_invalid_capacity_and_out_of_range_timestamps() {
        assert!(matches!(
            DurableReplayStore::open_in_memory(0),
            Err(ReplayStoreError::Protocol(
                ProtocolError::InvalidReplayGuardCapacity
            ))
        ));

        let mut store = DurableReplayStore::open_in_memory(4).unwrap();
        let order = verified('1', 'a', 'b', u64::MAX);
        expect_protocol_error(
            store.accept(&order, 150),
            ProtocolError::ReplayTimestampOutOfRange,
        );
        expect_protocol_error(
            store.accept(&verified('2', 'a', 'c', 300), i64::MAX as u64 + 1),
            ProtocolError::ReplayTimestampOutOfRange,
        );
        assert!(store.is_empty().unwrap());
    }
}
