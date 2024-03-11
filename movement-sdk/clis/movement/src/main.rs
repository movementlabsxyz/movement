#![forbid(unsafe_code)]

use clap::*;
use movement::Movement;
use util::cli::Command;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
   
   // todo: parse clap 
   let movement = Movement::parse();

   let res = movement.command.execute().await;
   
   println!("res: {:?}", res);

   Ok(())

}