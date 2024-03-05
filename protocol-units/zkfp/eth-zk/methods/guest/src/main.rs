#![no_main]

pub mod evm_integration;
use risc0_zkvm::guest::env;

risc0_zkvm::guest::entry!(main);

fn main() {
    let input: Vec<u8> = env::read();

    //run the VM and unwrap the output 
    let output = evm_integration::run(
        input,
        // todo: support type arg decoding from env for guest
        vec![],
        vec![],
    ).unwrap();

    env::log(format!("The Solidity program output {:#?} ", output).as_str());

    env::commit(&output)
}