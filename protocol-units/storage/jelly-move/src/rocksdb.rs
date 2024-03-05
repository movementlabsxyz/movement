use rocksdb::{DB, Options, ColumnFamilyDescriptor};
use std::sync::{Arc, RwLock};
use jmt::{
    KeyHash,
    storage::{
        NodeBatch,
        TreeReader,
        TreeWriter
    }
};
use serde::{Serialize, Deserialize};  
use std::hash::Hash;  

#[derive(Debug, Clone)]
pub struct RocksdbJmt {
    // [`rocksdb::DB`] is already interior mutably locked, so we don't need to wrap it in an `RwLock`
    db: Arc<DB>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Key {
    version : u64,
    key_hash : KeyHash
}

impl From<(u64, KeyHash)> for Key {
    fn from((version, key_hash): (u64, KeyHash)) -> Self {
        Key {
            version,
            key_hash
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
struct Value(Option<Vec<u8>>);

impl From<Option<Vec<u8>>> for Value {
    fn from(value: Option<Vec<u8>>) -> Self {
        Value(value)
    }
}

impl RocksdbJmt {
    pub fn try_new(path: &str) -> Result<Self, anyhow::Error> {
        let mut options = Options::default();
        options.create_if_missing(true);
        options.create_missing_column_families(true);

        let jmt_cf = ColumnFamilyDescriptor::new("jmt", Options::default());

        let db = DB::open_cf_descriptors(&options, path, vec![jmt_cf])
        .expect("Failed to open database with column families");

        Ok(RocksdbJmt {
            db: Arc::new(db),
        })
    }

    pub fn new(path: &str) -> Self {
        Self::try_new(path).expect("Failed to open database with column families")
    }

}

impl TreeWriter for RocksdbJmt {
    fn write_node_batch(&self, node_batch: &NodeBatch) -> anyhow::Result<()> {


        // get the cf handle or map the option to an error
        let cf_handle = self.db.cf_handle("jmt").ok_or(anyhow::anyhow!("Failed to get column family handle"))?;

        for (key, value) in node_batch.nodes() {
            self.db.put_cf(
                &cf_handle, 
                serde_json::to_vec(key)?,
                serde_json::to_vec(value)?,
            )?;
        }
        Ok(())
    }
}

impl TreeReader for RocksdbJmt {
    
    fn get_node(&self, node_key: &jmt::storage::NodeKey) -> anyhow::Result<jmt::storage::Node> {
      self.get_node_option(node_key)?.ok_or(anyhow::anyhow!("Failed to get node"))
    }

    fn get_node_option(&self, node_key: &jmt::storage::NodeKey) -> anyhow::Result<Option<jmt::storage::Node>> {
        let cf_handle = self.db.cf_handle("jmt").ok_or(anyhow::anyhow!("Failed to get column family handle"))?;
        let key = serde_json::to_vec(node_key)?;
        let value = self.db.get_cf(&cf_handle, key)?;
        match value {
            Some(value) => {
                let value: Option<
                jmt::storage::Node
                > = serde_json::from_slice(&value)?;
                Ok(value)
            }
            None => Ok(None)
        }
    }

    fn get_root_hash(&self) -> anyhow::Result<Option<jmt::KeyHash>> {
        let cf_handle = self.db.cf_handle("jmt").ok_or(anyhow::anyhow!("Failed to get column family handle"))?;
        let key = serde_json::to_vec(&"root_hash")?;
        let value = self.db.get_cf(&cf_handle, key)?;
        match value {
            Some(value) => {
                let value: Option<jmt::KeyHash> = serde_json::from_slice(&value)?;
                Ok(value)
            }
            None => Ok(None)
        }
    }

}