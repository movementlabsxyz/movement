use move_core_types::value;
use rocksdb::{DB, Options, ColumnFamilyDescriptor, BoundColumnFamily};
use std::sync::{Arc, RwLock};
use jmt::{
    KeyHash,
    Version,
    storage::{
        NodeBatch,
        TreeReader,
        TreeWriter,
        HasPreimage,
        TreeUpdateBatch,
        StaleNodeIndex
    }
};
use borsh::{BorshDeserialize, BorshSerialize};

#[derive(Debug, Clone)]
pub struct RocksdbJmt {
    // [`rocksdb::DB`] is already interior mutably locked, so we don't need to wrap it in an `RwLock`
    db: Arc<DB>
}

impl RocksdbJmt {

    const NODES_CF: &'static str = "nodes";
    const VALUE_HISTORY_CF: &'static str = "value_history";
    const STALE_NODE_CF: &'static str = "stale_nodes";
    const PREIMAGES_CF: &'static str = "preimages";

    pub fn try_new(path: &str) -> Result<Self, anyhow::Error> {
        let mut options = Options::default();
        options.create_if_missing(true);
        options.create_missing_column_families(true);

        // create column families
        let nodes_cf = ColumnFamilyDescriptor::new(Self::NODES_CF, Options::default());
        let value_history_cf = ColumnFamilyDescriptor::new(Self::VALUE_HISTORY_CF, Options::default());
        let stale_node_cf = ColumnFamilyDescriptor::new(Self::STALE_NODE_CF, Options::default());
        let preimages_cf = ColumnFamilyDescriptor::new(Self::PREIMAGES_CF, Options::default());
        let db = DB::open_cf_descriptors(&options, path, vec![
            nodes_cf,
            value_history_cf,
            stale_node_cf,
            preimages_cf
        ])
        .expect("Failed to open database with column families");

        Ok(RocksdbJmt {
            db: Arc::new(db),
        })
    }

    pub fn new(path: &str) -> Self {
        Self::try_new(path).expect("Failed to open database with column families")
    }

    pub fn nodes_cf(&self) -> Result<Arc<BoundColumnFamily>, anyhow::Error> {

        let cf = self.db.cf_handle(Self::NODES_CF).ok_or(anyhow::anyhow!("Failed to get column family handle"))?;
        Ok(cf)

    }

    pub fn value_history_cf(&self) -> Result<Arc<BoundColumnFamily>, anyhow::Error> {

        let cf = self.db.cf_handle(Self::VALUE_HISTORY_CF).ok_or(anyhow::anyhow!("Failed to get column family handle"))?;
        Ok(cf)

    }

    pub fn stale_nodes_cf(&self) -> Result<Arc<BoundColumnFamily>, anyhow::Error> {

        let cf = self.db.cf_handle(Self::STALE_NODE_CF).ok_or(anyhow::anyhow!("Failed to get column family handle"))?;
        Ok(cf)

    }

    pub fn preimages_cf(&self) -> Result<Arc<BoundColumnFamily>, anyhow::Error> {

        let cf = self.db.cf_handle(Self::PREIMAGES_CF).ok_or(anyhow::anyhow!("Failed to get column family handle"))?;
        Ok(cf)

    }


}

// https://github.com/penumbra-zone/jmt/blob/041ad5c7f6dfb9e2e16e09cf087e19c99008cc59/src/mock.rs#L98
impl TreeWriter for RocksdbJmt {
    fn write_node_batch(&self, node_batch: &NodeBatch) -> anyhow::Result<()> {

        // write nodes
        let cf_handle = self.nodes_cf()?;
        for (key, value) in node_batch.nodes() {
            self.db.put_cf(
                &cf_handle, 
                borsh::to_vec(key)?,
                borsh::to_vec(value)?,
            )?;
        }

        /// Write value history
        // todo: Place a value into the provided value history map. Versions must be pushed in non-decreasing order per key.
        // todo: determine whether the above is true for our implementation
        let cf_handle = self.value_history_cf()?;
        for (key, value) in node_batch.values() {
            self.db.put_cf(
                &cf_handle, 
                borsh::to_vec(key)?,
                borsh::to_vec(value)?,
            )?;
        }

        Ok(())
    }
}

impl TreeReader for RocksdbJmt {

    fn get_node_option(&self, node_key: &jmt::storage::NodeKey) -> anyhow::Result<Option<jmt::storage::Node>> {
        let cf_handle = self.nodes_cf()?;
        let key = borsh::to_vec(node_key)?;
        let value = self.db.get_cf(&cf_handle, key)?;
        match value {
            Some(value) => {
                let value = jmt::storage::Node::try_from_slice(&value)?;
                Ok(Some(value))
            }
            None => Ok(None)
        }
    }

    // https://github.com/penumbra-zone/jmt/blob/041ad5c7f6dfb9e2e16e09cf087e19c99008cc59/src/mock.rs#L73
    fn get_value_option(
            &self,
            max_version: jmt::Version,
            key_hash: KeyHash,
        ) -> anyhow::Result<Option<jmt::OwnedValue>> {
        let value_history_cf = self.value_history_cf()?;
        
        // move backwards from max_version (inclusive) until we find a value
        for version in (0..max_version + 1).rev() {
            let key = (version, key_hash.clone());
            let value = self.db.get_cf(&value_history_cf, borsh::to_vec(&key)?)?;
            if let Some(value) = value {
                let value : Option<jmt::OwnedValue> = BorshDeserialize::try_from_slice(&value)?;
                return Ok(value);
            }
        }

        Ok(None)

    }

    fn get_rightmost_leaf(&self) -> anyhow::Result<Option<(jmt::storage::NodeKey, jmt::storage::LeafNode)>> {
        // todo: not sure if this is really the right most leaf
        let cf_handle = self.nodes_cf()?;
        let mut iter = self.db.iterator_cf(&cf_handle, rocksdb::IteratorMode::End);
        let (key, value) = iter.next().ok_or(anyhow::anyhow!("Failed to get rightmost leaf"))??;
        let key = jmt::storage::NodeKey::try_from_slice(&key)?;
        let value = jmt::storage::LeafNode::try_from_slice(&value)?;
        Ok(Some((key, value)))
    }

}

impl HasPreimage for RocksdbJmt {
    fn preimage(&self, key_hash: KeyHash) -> anyhow::Result<Option<Vec<u8>>> {
        let cf_handle = self.preimages_cf()?;
        let value = self.db.get_cf(&cf_handle, borsh::to_vec(&key_hash)?)?;
        Ok(value)
    }
}

// Useful operations for actually writing to the database
/// todo: extract into trait, maybe we also want to fork jmt
impl RocksdbJmt {

    pub fn write_tree_update_batch(&self, batch: &TreeUpdateBatch) -> Result<(), anyhow::Error> {

        // write the node batch
        self.write_node_batch(&batch.node_batch)?;

        // write the stale nodes
        let cf_handle = self.stale_nodes_cf()?;
        for stale_node_index in batch.stale_node_index_batch.iter() {
            self.db.put_cf(&cf_handle, borsh::to_vec(&stale_node_index)?, &[])?;
        }

        Ok(())
    }
   
   pub fn purge_stale_nodes(&self, last_readable_version: Version) -> Result<(), anyhow::Error> {

        let cf_handle = self.stale_nodes_cf()?;
        let mut iter = self.db.iterator_cf(&cf_handle, rocksdb::IteratorMode::Start);
        while let Some(res) = iter.next() {
            let (key, _) = res?;
            let key : (Version, KeyHash) = BorshDeserialize::try_from_slice(&key)?;
            if key.0 <= last_readable_version {
                // remove the key from the nodes column family
                let cf_handle = self.nodes_cf()?;
                self.db.delete_cf(&cf_handle, borsh::to_vec(&key.1)?)?;
                // remove the key from the stale nodes column family
                self.db.delete_cf(&cf_handle, borsh::to_vec(&key)?)?;
            }
        }

        Ok(())

   }

   pub fn num_nodes(&self) -> usize {
        let cf_handle = self.nodes_cf().unwrap();
        let mut iter = self.db.iterator_cf(&cf_handle, rocksdb::IteratorMode::Start);
        let mut count = 0;
        while let Some(_) = iter.next() {
            count += 1;
        }
        count
   }

}

pub mod test {

    use super::*;
    use std::collections::BTreeMap;
    use jmt::{
        JellyfishMerkleTree,
        SimpleHasher,
        storage::{
            Node,
            NodeKey
        }
    };
    use sha2::Sha256;
    use tempfile::TempDir;

    fn update_nibble(original_key: &KeyHash, n: usize, nibble: u8) -> KeyHash {
        assert!(nibble < 16);
        let mut key = original_key.0;
        key[n / 2] = if n % 2 == 0 {
            key[n / 2] & 0x0f | nibble << 4
        } else {
            key[n / 2] & 0xf0 | nibble
        };
        KeyHash(key)
    }

    #[test]
    fn test_rocksdb_jmt() -> Result<(), anyhow::Error> {
            
        let dir = TempDir::new()?;
        let db = RocksdbJmt::try_new(dir.path().to_str().unwrap())?;
        let tree = JellyfishMerkleTree::<RocksdbJmt, Sha256>::new(&db);

        let key = b"testkey";
        let value = vec![1u8, 2u8, 3u8, 4u8];

        // batch version
        let (_new_root_hash, batch) = tree
            .batch_put_value_sets(
                vec![vec![(KeyHash::with::<Sha256>(key), value.clone())]],
                None,
                0, /* version */
            )
            .unwrap();
        assert!(batch.stale_node_index_batch.is_empty());

        db.write_tree_update_batch(&batch)?;

        assert_eq!(
            tree.get(KeyHash::with::<Sha256>(key), 0)?.ok_or(anyhow::anyhow!("Failed to get value"))?,
            value
        );

        Ok(())
        
    }

    #[test]
    fn test_insert_at_leaf_with_internal_created()  -> Result<(), anyhow::Error> {
        let dir = TempDir::new()?;
        let db = RocksdbJmt::try_new(dir.path().to_str().unwrap())?;
        let tree = JellyfishMerkleTree::<_, Sha256>::new(&db);
    
        let key1 = KeyHash([0u8; 32]);
        let value1 = vec![1u8, 2u8];
    
        let (_root0_hash, batch) = tree
            .batch_put_value_sets(
                vec![vec![(key1, value1.clone())]],
                None,
                0, /* version */
            )
            .unwrap();
    
        assert!(batch.stale_node_index_batch.is_empty());
        db.write_tree_update_batch(&batch).unwrap();
        assert_eq!(tree.get(key1, 0).unwrap().unwrap(), value1);
    
        // Insert at the previous leaf node. Should generate an internal node at the root.
        // Change the 1st nibble to 15.
        let key2 = update_nibble(&key1, 0, 15);
        let value2 = vec![3u8, 4u8];
    
        let (_root1_hash, batch) = tree
            .batch_put_value_sets(
                vec![vec![(key2, value2.clone())]],
                None,
                1, /* version */
            )
            .unwrap();
        assert_eq!(batch.stale_node_index_batch.len(), 1);
        db.write_tree_update_batch(&batch).unwrap();
    
        assert_eq!(tree.get(key1, 0).unwrap().unwrap(), value1);
        assert!(tree.get(key2, 0).unwrap().is_none());
        assert_eq!(tree.get(key2, 1).unwrap().unwrap(), value2);
    
        // get # of nodes
        assert_eq!(db.num_nodes(), 4 /* 1 + 3 */);
    
        Ok(())

    }

}