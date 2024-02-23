#![no_main]
// If you want to try std support, also update the guest Cargo.toml file

pub mod move_vm_integration_run;


use risc0_zkvm::guest::env;

risc0_zkvm::guest::entry!(main);


fn main() {

    // read the input
    let input: Vec<u8> = env::read();

    // run the VM and unwrap the output
    let output = move_vm_integration_run::run(
        input,
        // todo: support type arg decoding from env for guest
        vec![],
        vec![],
    ).unwrap();

    env::log(format!("The move program output {:#?} ", output).as_str());

    env::commit(&output);
}
