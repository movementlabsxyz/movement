use sui_helper_types::{
    providers::object_version::ObjectVersionProvider,
    block::VerifiedExecutableExecutionGroups
};

#[derive(Debug, Clone)] // would like for this to remain debug clone
pub struct ObjectVersionRocksDB {
    
}

#[async_trait::async_trait]
impl ObjectVersionProvider for ObjectVersionRocksDB {
    
    // todo: implement against rocksdb store
    // todo: add methods to the trait in the `sui-helper-types` crate

    async fn assign_shared_object_versions(&self, transactions : VerifiedExecutableExecutionGroups) -> Result<VerifiedExecutableExecutionGroups, anyhow::Error> {
        unimplemented!();
    }

}