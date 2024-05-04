use move_core_types::effects::ChangeSet;

pub trait ChangeSetWriter {

    /// Write a change set to storage
    fn write_change_set(&self, change_set: ChangeSet) -> Result<(), anyhow::Error>;

}