use crate::compiler::{as_module, compile_units};
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

TEST_ADDR: AccountAddress = AccountAddress::new([42; AccountAddress::LENGTH]);

/// Makes a foo function that is compatible with the expectations of the guest code. 
pub fn make_foo(
    structs: &[&str],
    fun_sig: &str,
    fun_body: &str,
)->Vec<u8> {
    let structs = structs.to_vec().join("\n");

    let code = format!(
        r#"
        module 0x{}::M {{
            {}

            fun foo{} {{
                {}
            }}
        }}
    "#,
        TEST_ADDR, structs, fun_sig, fun_body
    );

    let mut units = compile_units(&code).unwrap();
    let m = as_module(units.pop().unwrap());
    let mut blob = vec![];
    m.serialize(&mut blob).unwrap();
}

/// Uses make foo to make a very simple function that returns a u64 (42)
pub fn return_u64() -> Vec<u8> {
    make_foo(&[], "(): u64", "42")
}