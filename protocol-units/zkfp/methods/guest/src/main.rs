#![no_main]
// If you want to try std support, also update the guest Cargo.toml file

// pub mod move_vm_integration_run;
use aptos_vm::AptosVM;
use aptos_executor::block_executor::{self, BlockExecutor};
use aptos_storage_interface::{mock::MockDbReaderWriter, DbReaderWriter, DbReader, DbWriter};

risc0_zkvm::guest::entry!(main);

async fn does_this_even_compile() -> u8 {
    42
}

fn main() {

    let mock = MockDbReaderWriter;

    let block_executor = BlockExecutor::<AptosVM>::new(
        DbReaderWriter::new(mock),
    );


}
