use move_core_types::language_storage::ModuleId;

pub trait BasicStorageOperations {

    fn publish_or_overwrite_module(&self, id: ModuleId, blob: Vec<u8>) -> Result<(), anyhow::Error>;

}