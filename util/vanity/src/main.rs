mod cli;
mod miner;

use std::str::FromStr;
use clap::Parser;
use cli::{Pattern, Vanity};
use miner::mine_move_address;
use num_cpus;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    match Vanity::parse() {
        Vanity::Move { starts_pattern, ends_pattern } => {
            let starts_pattern_bytes = match starts_pattern {
                Some(p) => Pattern::from_str(&p)?.into_bytes()?,
                None => vec![],
            };
            let ends_pattern_bytes = match ends_pattern {
                Some(p) => Pattern::from_str(&p)?.into_bytes()?,
                None => vec![],
            };

            let account = mine_move_address(
                &starts_pattern_bytes,
                &ends_pattern_bytes,
                num_cpus::get(),
            );

            println!("Found Move address: {}", account.address());
            println!("Private key (hex): {}", hex::encode(account.private_key().to_bytes()));
            return Ok(());
        }
    }
}
