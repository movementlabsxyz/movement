use move_binary_format::errors::VMResult;
use move_core_types::{
    account_address::AccountAddress,
    identifier::Identifier,
    language_storage::{ModuleId, TypeTag},
    value::{MoveTypeLayout, MoveValue},
};
use move_vm_runtime::{move_vm::MoveVM, session::SerializedReturnValues};
use move_vm_test_utils::InMemoryStorage;
use move_vm_types::gas::UnmeteredGasMeter;

const TEST_ADDR: AccountAddress = AccountAddress::new([42; AccountAddress::LENGTH]);

pub fn run(
    blob: Vec<u8>,
    ty_args: Vec<TypeTag>,
    args: Vec<MoveValue>,
) -> VMResult<Vec<Vec<u8>>> {

    let mut storage = InMemoryStorage::new();
    let module_id = ModuleId::new(TEST_ADDR, Identifier::new("M").unwrap());
    storage.publish_or_overwrite_module(module_id.clone(), blob);

    let vm = MoveVM::new(vec![]).unwrap();
    let mut sess = vm.new_session(&storage);

    let fun_name = Identifier::new("foo").unwrap();

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
        ty_args,
        args,
        &mut UnmeteredGasMeter,
    )?;

    Ok(return_values
        .into_iter()
        .map(|(bytes, _layout)| bytes)
        .collect())
}
