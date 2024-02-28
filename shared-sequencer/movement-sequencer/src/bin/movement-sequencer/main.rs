pub mod genesis;
pub mod vm_id;

use std::io;

use avalanche_types::subnet;
use clap::{crate_version, Command};
use movement_sequencer::vm;
use tokio::sync::broadcast::{self, Receiver, Sender};

pub const APP_NAME: &str = "movement-sequencer";

#[tokio::main]
async fn main() -> io::Result<()> {
    let matches = Command::new(APP_NAME)
        .version(crate_version!())
        .about("Movement Sequencer")
        .subcommands(vec![genesis::command(), vm_id::command()])
        .get_matches();

    // ref. https://github.com/env-logger-rs/env_logger/issues/47
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

    match matches.subcommand() {
        Some((genesis::NAME, sub_matches)) => {
            let data = sub_matches.get_one::<String>("DATA").expect("required");
            let genesis = movement_sequencer::genesis::Genesis { data: data.clone() };
            println!("{genesis}");

            Ok(())
        }

        Some((vm_id::NAME, sub_matches)) => {
            let vm_name = sub_matches.get_one::<String>("VM_NAME").expect("required");
            let id = subnet::vm_name_to_id(vm_name)?;
            println!("{id}");

            Ok(())
        }

        _ => {
            log::info!("starting movement-sequencer");

            let (stop_ch_tx, stop_ch_rx): (Sender<()>, Receiver<()>) = broadcast::channel(1);
            let vm_server = subnet::rpc::vm::server::Server::new(vm::Vm::new(), stop_ch_tx);
            subnet::rpc::vm::serve(vm_server, stop_ch_rx).await
        }
    }
}
