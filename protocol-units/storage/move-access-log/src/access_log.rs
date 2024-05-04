use move_core_types::{
    account_address::AccountAddress, 
    identifier::Identifier, 
    language_storage::{ModuleId, StructTag},
    resolver::{ModuleResolver, ResourceResolver}
};  
use std::collections::BTreeSet;
use move_vm_ext::storage::{
    ChangeSetWriter,
    BasicStorageOperations
};
// refcell for interior mutability
use std::cell::RefCell;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Read {
    ModuleId(ModuleId),
    AccountAddress(AccountAddress),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Write {
    Module((AccountAddress, Identifier)),
    Resource((AccountAddress, StructTag))
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Access {
    Read(Read),
    Write(Write)
}

#[derive(Debug, Clone)]
pub struct AccessLog {
    pub accesses : BTreeSet<Access>
}

impl AccessLog {
    pub fn new() -> Self {
        Self {
            accesses : BTreeSet::new()
        }
    }
}

pub struct WithAccessLog<T> {
    pub storage : T,
    // interior mutability on the access log
    pub access_log : RefCell<AccessLog>
}

impl<T : ChangeSetWriter + ModuleResolver + ResourceResolver> WithAccessLog<T> {

    pub fn new(storage : T) -> Self {
        Self {
            storage,
            access_log : RefCell::new(AccessLog::new())
        }
    }

}

impl <T> WithAccessLog<T> {

    pub fn get_access_log(&self) -> BTreeSet<Access> {
        self.access_log.borrow().accesses.clone()
    }

    pub fn clear_access_log(&self) {
        self.access_log.borrow_mut().accesses.clear()
    }

    pub fn log_change_set(&self, change_set : &move_core_types::effects::ChangeSet) -> Result<(), anyhow::Error> {

        for (addr, id , _) in change_set.modules() {
            self.access_log.borrow_mut().accesses.insert(Access::Write(Write::Module((addr, id.clone()))));
        }

        for (addr, tag, _) in change_set.resources() {
            self.access_log.borrow_mut().accesses.insert(Access::Write(Write::Resource((addr, tag.clone()))));
        }

        Ok(())
    }

}

impl <T : ModuleResolver> ModuleResolver for WithAccessLog<T> {
    
    type Error = T::Error;

    fn get_module(&self, id: &ModuleId) -> Result<Option<Vec<u8>>, Self::Error> {
        self.access_log.borrow_mut().accesses.insert(Access::Read(Read::ModuleId(id.clone())));
        self.storage.get_module(id)
    }

}

impl <T : ResourceResolver> ResourceResolver for WithAccessLog<T> {
    
    type Error = T::Error;

    fn get_resource(&self, addr: &AccountAddress, tag: &StructTag) -> Result<Option<Vec<u8>>, Self::Error> {
        self.access_log.borrow_mut().accesses.insert(Access::Read(Read::AccountAddress(addr.clone())));
        self.storage.get_resource(addr, tag)
    }

}

impl <T : ChangeSetWriter> ChangeSetWriter for WithAccessLog<T> {
    
    fn write_change_set(&self, change_set: move_core_types::effects::ChangeSet) -> Result<(), anyhow::Error> {

        self.log_change_set(&change_set)?;

        self.storage.write_change_set(change_set)
    }

}

impl <T : BasicStorageOperations> BasicStorageOperations for WithAccessLog<T> {
    
    fn publish_or_overwrite_module(&self, id: ModuleId, blob: Vec<u8>) -> Result<(), anyhow::Error> {
        self.storage.publish_or_overwrite_module(id, blob)
    }

}