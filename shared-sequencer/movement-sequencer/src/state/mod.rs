//! Manages the virtual machine states.

use std::{
    collections::HashMap,
    io::{self, Error, ErrorKind},
    sync::Arc,
};

use crate::block::Block;
use avalanche_types::{choices, ids, subnet};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

/// Manages block and chain states for this Vm, both in-memory and persistent.
#[derive(Clone)]
pub struct State {
    pub db: Arc<RwLock<Box<dyn subnet::rpc::database::Database + Send + Sync>>>,

    /// Maps block Id to Block.
    /// Each element is verified but not yet accepted/rejected (e.g., preferred).
    pub verified_blocks: Arc<RwLock<HashMap<ids::Id, Block>>>,
}

impl Default for State {
    fn default() -> State {
        Self {
            db: Arc::new(RwLock::new(subnet::rpc::database::memdb::Database::new())),
            verified_blocks: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

const LAST_ACCEPTED_BLOCK_KEY: &[u8] = b"last_accepted_block";

const STATUS_PREFIX: u8 = 0x0;

const DELIMITER: u8 = b'/';

/// Returns a vec of bytes used as a key for identifying blocks in state.
/// '`STATUS_PREFIX`' + '`BYTE_DELIMITER`' + [`block_id`]
fn block_with_status_key(blk_id: &ids::Id) -> Vec<u8> {
    let mut k: Vec<u8> = Vec::with_capacity(ids::LEN + 2);
    k.push(STATUS_PREFIX);
    k.push(DELIMITER);
    k.extend_from_slice(&blk_id.to_vec());
    k
}

/// Wraps a [`Block`](crate::block::Block) and its status.
/// This is the data format that [`State`](State) uses to persist blocks.
#[derive(Serialize, Deserialize, Clone)]
struct BlockWithStatus {
    block_bytes: Vec<u8>,
    status: choices::status::Status,
}

impl BlockWithStatus {
    fn encode(&self) -> io::Result<Vec<u8>> {
        serde_json::to_vec(&self).map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("failed to serialize BlockStatus to JSON bytes: {e}"),
            )
        })
    }

    fn from_slice(d: impl AsRef<[u8]>) -> io::Result<Self> {
        let dd = d.as_ref();
        serde_json::from_slice(dd).map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("failed to deserialize BlockStatus from JSON: {e}"),
            )
        })
    }
}

impl State {
    /// Persists the last accepted block Id to state.
    /// # Errors
    /// Fails if the db can't be updated
    pub async fn set_last_accepted_block(&self, blk_id: &ids::Id) -> io::Result<()> {
        let mut db = self.db.write().await;
        db.put(LAST_ACCEPTED_BLOCK_KEY, &blk_id.to_vec())
            .await
            .map_err(|e| {
                Error::new(
                    ErrorKind::Other,
                    format!("failed to put last accepted block: {e:?}"),
                )
            })
    }

    /// Returns "true" if there's a last accepted block found.
    /// # Errors
    /// Fails if the db can't be read
    pub async fn has_last_accepted_block(&self) -> io::Result<bool> {
        let db = self.db.read().await;
        match db.has(LAST_ACCEPTED_BLOCK_KEY).await {
            Ok(found) => Ok(found),
            Err(e) => Err(Error::new(
                ErrorKind::Other,
                format!("failed to load last accepted block: {e}"),
            )),
        }
    }

    /// Returns the last accepted block Id from state.
    /// # Errors
    /// Can fail if the db can't be read
    pub async fn get_last_accepted_block_id(&self) -> io::Result<ids::Id> {
        let db = self.db.read().await;
        match db.get(LAST_ACCEPTED_BLOCK_KEY).await {
            Ok(d) => Ok(ids::Id::from_slice(&d)),
            Err(e) => {
                if subnet::rpc::errors::is_not_found(&e) {
                    return Ok(ids::Id::empty());
                }
                Err(e)
            }
        }
    }

    /// Adds a block to "`verified_blocks`".
    pub async fn add_verified(&mut self, block: &Block) {
        let blk_id = block.id();
        log::info!("verified added {blk_id}");

        let mut verified_blocks = self.verified_blocks.write().await;
        verified_blocks.insert(blk_id, block.clone());
    }

    /// Removes a block from "`verified_blocks`".
    pub async fn remove_verified(&mut self, blk_id: &ids::Id) {
        let mut verified_blocks = self.verified_blocks.write().await;
        verified_blocks.remove(blk_id);
    }

    /// Returns "true" if the block Id has been already verified.
    pub async fn has_verified(&self, blk_id: &ids::Id) -> bool {
        let verified_blocks = self.verified_blocks.read().await;
        verified_blocks.contains_key(blk_id)
    }

    /// Writes a block to the state storage.
    /// # Errors
    /// Can fail if the block fails to serialize or if the db can't be updated
    pub async fn write_block(&mut self, block: &Block) -> io::Result<()> {
        let blk_id = block.id();
        let blk_bytes = block.to_vec()?;

        let mut db = self.db.write().await;

        let blk_status = BlockWithStatus {
            block_bytes: blk_bytes,
            status: block.status(),
        };
        let blk_status_bytes = blk_status.encode()?;

        db.put(&block_with_status_key(&blk_id), &blk_status_bytes)
            .await
            .map_err(|e| Error::new(ErrorKind::Other, format!("failed to put block: {e:?}")))
    }

    /// Reads a block from the state storage using the `block_with_status_key`.
    /// # Errors
    /// Can fail if the block is not found in the state storage, or if the block fails to deserialize
    pub async fn get_block(&self, blk_id: &ids::Id) -> io::Result<Block> {
        // check if the block exists in memory as previously verified.
        let verified_blocks = self.verified_blocks.read().await;
        if let Some(b) = verified_blocks.get(blk_id) {
            return Ok(b.clone());
        }

        let db = self.db.read().await;

        let blk_status_bytes = db.get(&block_with_status_key(blk_id)).await?;
        let blk_status = BlockWithStatus::from_slice(blk_status_bytes)?;

        let mut blk = Block::from_slice(&blk_status.block_bytes)?;
        blk.set_status(blk_status.status);

        Ok(blk)
    }
}

/// RUST_LOG=debug cargo test --package timestampvm --lib -- state::test_state --exact --show-output
#[tokio::test]
async fn test_state() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .is_test(true)
        .try_init();

    let genesis_blk = Block::try_new(
        ids::Id::empty(),
        0,
        random_manager::u64(),
        random_manager::secure_bytes(10).unwrap(),
        choices::status::Status::Accepted,
    )
    .unwrap();
    log::info!("genesis block: {genesis_blk}");

    let blk1 = Block::try_new(
        genesis_blk.id(),
        1,
        genesis_blk.timestamp() + 1,
        random_manager::secure_bytes(10).unwrap(),
        choices::status::Status::Accepted,
    )
    .unwrap();
    log::info!("blk1: {blk1}");

    let mut state = State::default();
    assert!(!state.has_last_accepted_block().await.unwrap());

    state.write_block(&genesis_blk).await.unwrap();
    assert!(!state.has_last_accepted_block().await.unwrap());

    state.write_block(&blk1).await.unwrap();
    state.set_last_accepted_block(&blk1.id()).await.unwrap();
    assert!(state.has_last_accepted_block().await.unwrap());

    let last_accepted_blk_id = state.get_last_accepted_block_id().await.unwrap();
    assert_eq!(last_accepted_blk_id, blk1.id());

    let read_blk = state.get_block(&genesis_blk.id()).await.unwrap();
    assert_eq!(genesis_blk, read_blk);

    let read_blk = state.get_block(&blk1.id()).await.unwrap();
    assert_eq!(blk1, read_blk);
}
