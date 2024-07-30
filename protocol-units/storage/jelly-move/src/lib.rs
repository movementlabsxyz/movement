pub mod types;
pub mod rocksdb;

use anyhow::Ok;
use move_core_types::{
    account_address:: AccountAddress,
    language_storage::{ModuleId, StructTag},
    resolver::{
        ModuleResolver, ResourceResolver
    },
    effects::{
        ChangeSet,
        Event,
        Op
    }
};
use jmt::{
    proof::SparseMerkleProof, storage::{NodeKey, NodeBatch, TreeReader, TreeWriter}, JellyfishMerkleTree, KeyHash, OwnedValue, SimpleHasher
};
use rocksdb::RocksdbJmt;
use serde_json::ser;

#[derive()]
pub struct JellyMove<'a, R : 'a + TreeReader, H : SimpleHasher> {
    jmt: JellyfishMerkleTree<'a, R, H>,
    writer: &'a dyn TreeWriter
}

impl <'a, R : 'a + TreeReader, H : SimpleHasher> JellyMove<'a, R, H> {

    const MODULE_PREFIX: &'static str = "MODULE::";
    const RESOURCE_PREFIX: &'static str = "RESOURCE::";

    pub fn new(jmt: JellyfishMerkleTree<'a, R, H>, writer: &'a dyn TreeWriter) -> Self {
        Self {
            jmt,
            writer
        }
    }

    pub fn get_latest_version(&self) -> Result<u64, anyhow::Error> {
        Ok(0)
    }

    pub fn module_key(&self, id: &ModuleId) -> Result<Vec<u8>, anyhow::Error> {
        let mut key = Vec::new();
        key.extend_from_slice(Self::MODULE_PREFIX.as_bytes());
        key.extend_from_slice(
            serde_json::to_vec(id)?.as_slice()
        );
        Ok(key)
    }

    pub fn resource_key(
        &self,
        account_address : &AccountAddress,
        tag: &StructTag
    ) -> Result<Vec<u8>, anyhow::Error> {

        let mut key = Vec::new();
        key.extend_from_slice(Self::RESOURCE_PREFIX.as_bytes());
        key.extend_from_slice(
            serde_json::to_vec(account_address)?.as_slice()
        );
        key.extend_from_slice(
            serde_json::to_vec(tag)?.as_slice()
        );
        Ok(key)

    }

}

impl <'a, R : 'a + TreeReader, H : SimpleHasher> ModuleResolver for JellyMove<'a, R, H>{

    type Error = anyhow::Error;

    fn get_module(&self, id: &ModuleId) -> Result<Option<Vec<u8>>, Self::Error> {

        let key = self.module_key(id)?;

        let value = self.jmt.get(
            KeyHash::with::<H>(&key),
            self.get_latest_version()?
        )?;

        Ok(value)

    }

}

impl <'a, R : 'a + TreeReader, H : SimpleHasher> JellyMove<'a, R, H> {

    pub fn get_module_with_proof(&self, id: &ModuleId) -> Result<Option<(Vec<u8>, SparseMerkleProof<H>)>, anyhow::Error> {

        let key = self.module_key(id)?;

        let (value, proof) = self.jmt.get_with_proof(
            KeyHash::with::<H>(&key),
            self.get_latest_version()?
        )?;

        Ok(value.map(|v| (v, proof)))

    }

}


impl <'a, R : 'a + TreeReader, H : SimpleHasher> ResourceResolver for JellyMove<'a, R, H> {

    type Error = anyhow::Error;

    fn get_resource(
        &self,
        account_address: &AccountAddress,
        tag: &StructTag
    ) -> Result<Option<Vec<u8>>, Self::Error> {

        let key = self.resource_key(account_address, tag)?;

        let value = self.jmt.get(
            KeyHash::with::<H>(&key),
            self.get_latest_version()?
        )?;

        Ok(value)

    }

}

impl <'a, R : 'a + TreeReader, H : SimpleHasher> JellyMove<'a, R, H> {

    pub fn get_resource_with_proof(
        &self,
        account_address: &AccountAddress,
        tag: &StructTag
    ) -> Result<Option<(Vec<u8>, SparseMerkleProof<H>)>, anyhow::Error> {

        let key = self.resource_key(account_address, tag)?;

        let (value, proof) = self.jmt.get_with_proof(
            KeyHash::with::<H>(&key),
            self.get_latest_version()?
        )?;

        Ok(value.map(|v| (v, proof)))

    }
}

impl <'a, R : 'a + TreeReader, H : SimpleHasher> move_vm_ext::storage::ChangeSetWriter for JellyMove<'a, R, H> {

    fn write_change_set(&self, change_set: ChangeSet) -> Result<(), anyhow::Error> {

        let mut value_sets : Vec<(KeyHash, Option<OwnedValue>)> = Vec::new();

        for (account_address, identifier, value) in change_set.modules() {

            let module_id = ModuleId::new(account_address, identifier.clone());
            let key = self.module_key(&module_id)?;

            let key_hash = KeyHash::with::<H>(&key);

            value_sets.push((key_hash, value.ok().map(|v| v.to_owned())));

        }

        for (account_address, struct_tag, value) in change_set.resources() {

            let key = self.resource_key(&account_address, &struct_tag)?;

            let key_hash = KeyHash::with::<H>(&key);

            value_sets.push((key_hash, value.ok().map(|v| v.to_owned())));

        }

        let (root_hash, tree_update_batch) = self.jmt.put_value_set(
            value_sets,
            self.get_latest_version()?,
        )?;


        self.writer.write_node_batch(
           &tree_update_batch.node_batch
        )?;

        // todo: add stale node writing
        Ok(())
    }

}

impl <'a, R : 'a + TreeReader, H : SimpleHasher> move_vm_ext::storage::BasicStorageOperations for JellyMove<'a, R, H> {

    fn publish_or_overwrite_module(&self, id: ModuleId, blob: Vec<u8>) -> Result<(), anyhow::Error> {

        let key = self.module_key(&id)?;

        let key_hash = KeyHash::with::<H>(&key);

        let (root_hash, tree_update_batch) = self.jmt.put_value_set(
            vec![(key_hash, Some(blob.to_owned()))],
            self.get_latest_version()?
        )?;

        self.writer.write_node_batch(
            &tree_update_batch.node_batch
        )?;

        Ok(())

    }

}

pub mod test {
    use super::*;
    use std::{path::Path, vec};
    use serde::Serialize;
    use tempfile::TempDir;

    use move_binary_format::errors::VMResult;
    use move_core_types::{
        value::serialize_values,
        account_address::AccountAddress,
        identifier::Identifier,
        language_storage::{ModuleId, TypeTag},
        value::{MoveTypeLayout, MoveValue},
    };
    use move_vm_runtime::{move_vm::MoveVM, session::SerializedReturnValues};
    use move_vm_test_utils::InMemoryStorage;
    use move_vm_types::gas::UnmeteredGasMeter;
    use move_vm_integration_test_helpers::{
        compiler_examples,
        compiler
    };
    use move_vm_ext::storage::{ChangeSetWriter, BasicStorageOperations};

    const TEST_ADDR: AccountAddress = AccountAddress::new([42; AccountAddress::LENGTH]);

    #[test]
    fn test_call_published_module() -> Result<(), anyhow::Error> {

        let dir = TempDir::new()?;
        let jmt = RocksdbJmt::new(dir.path().to_str().unwrap());
        let storage : JellyMove<'_, RocksdbJmt, sha2::Sha256> = JellyMove::new(
            JellyfishMerkleTree::new(&jmt),
            &jmt
        );

        let blob = compiler_examples::return_u64();
        let module_id = ModuleId::new(TEST_ADDR, Identifier::new("M")?);

        storage.publish_or_overwrite_module(module_id.clone(), blob)?;

        let vm = MoveVM::new(vec![]).unwrap();
        let mut sess = vm.new_session(&storage);

        let fun_name = Identifier::new("foo").unwrap();

        let args : Vec<MoveValue> = vec![];
        let args: Vec<_> = args
        .into_iter()
        .map(|val| val.simple_serialize().unwrap())
        .collect();

        let SerializedReturnValues {
            return_values,
            mutable_reference_outputs: _,
        } = sess.execute_function_bypass_visibility(
            &module_id,
            &fun_name,
            vec![],
            args,
            &mut UnmeteredGasMeter,
        )?;

        Ok(())

    }

    #[test]
    fn test_mutate_account() -> Result<(), anyhow::Error> {

        let dir = TempDir::new()?;
        let jmt = RocksdbJmt::new(dir.path().to_str().unwrap());
        let storage : JellyMove<'_, RocksdbJmt, sha2::Sha256> = JellyMove::new(
            JellyfishMerkleTree::new(&jmt),
            &jmt
        );

        let code = r#"
            module {{ADDR}}::M {
                struct Foo has key { a: bool }
                public fun get(addr: address): bool acquires Foo {
                    borrow_global<Foo>(addr).a
                }
                public fun flip(addr: address) acquires Foo {
                    let f_ref = borrow_global_mut<Foo>(addr);
                    f_ref.a = !f_ref.a;
                }
                public fun publish(addr: &signer) {
                    move_to(addr, Foo { a: true} )
                }
            }
        "#;

        let code = code.replace("{{ADDR}}", &format!("0x{}", TEST_ADDR));
        let mut units = compiler::compile_units(&code)?;
        let m = compiler::as_module(units.pop().unwrap());
        let mut blob = vec![];
        m.serialize(&mut blob)?;

        let module_id = ModuleId::new(TEST_ADDR, Identifier::new("M").unwrap());
        storage.publish_or_overwrite_module(module_id.clone(), blob)?;

        let vm = MoveVM::new(vec![]).unwrap();
        let mut sess = vm.new_session(&storage);

        let publish = Identifier::new("publish").unwrap();
        let flip = Identifier::new("flip").unwrap();
        let get = Identifier::new("get").unwrap();

        let account1 = AccountAddress::random();

        sess.execute_function_bypass_visibility(
            &module_id,
            &publish,
            vec![],
            serialize_values(&vec![MoveValue::Signer(account1)]),
            &mut UnmeteredGasMeter,
        )?;

        // The resource was published to "account1" and the sender's account
        // (TEST_ADDR) is assumed to be mutated as well (e.g., in a subsequent
        // transaction epilogue).
        assert_eq!(sess.num_mutated_accounts(&TEST_ADDR), 2);

        sess.execute_function_bypass_visibility(
            &module_id,
            &get,
            vec![],
            serialize_values(&vec![MoveValue::Address(account1)]),
            &mut UnmeteredGasMeter,
        )?;

        let (change_set, _ ) = sess.finish()?;

        storage.write_change_set(change_set)?;

        // second session
        let mut sess_two = vm.new_session(&storage);

        let res = sess_two.execute_function_bypass_visibility(
            &module_id,
            &get,
            vec![],
            serialize_values(&vec![MoveValue::Address(account1)]),
            &mut UnmeteredGasMeter,
        )?;

        assert_eq!(res.return_values.len(), 1);
        assert_eq!(
            res.return_values[0].0,
            vec![0x01]
        );
        assert_eq!(
            res.return_values[0].1.to_string(),
            MoveTypeLayout::Bool.to_string()
        );

        // now flip the value
        sess_two.execute_function_bypass_visibility(
            &module_id,
            &flip,
            vec![],
            serialize_values(&vec![MoveValue::Address(account1)]),
            &mut UnmeteredGasMeter,
        )?;

        // get the value again
        let res = sess_two.execute_function_bypass_visibility(
            &module_id,
            &get,
            vec![],
            serialize_values(&vec![MoveValue::Address(account1)]),
            &mut UnmeteredGasMeter,
        )?;

        assert_eq!(res.return_values.len(), 1);
        assert_eq!(
            res.return_values[0].0,
            vec![0x00]
        );
        assert_eq!(
            res.return_values[0].1.to_string(),
            MoveTypeLayout::Bool.to_string()
        );

        // now finish the session
        let (change_set, _ ) = sess_two.finish()?;
        storage.write_change_set(change_set)?;

        // now start a third session
        let mut sess_three = vm.new_session(&storage);

        let res = sess_three.execute_function_bypass_visibility(
            &module_id,
            &get,
            vec![],
            serialize_values(&vec![MoveValue::Address(account1)]),
            &mut UnmeteredGasMeter,
        )?;

        assert_eq!(res.return_values.len(), 1);
        assert_eq!(
            res.return_values[0].0,
            vec![0x00]
        );
        assert_eq!(
            res.return_values[0].1.to_string(),
            MoveTypeLayout::Bool.to_string()
        );

        Ok(())

    }


}
