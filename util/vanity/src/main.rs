use vanity::{cli, miner};

use aptos_sdk::types::account_address::AccountAddress;
use clap::Parser;
use std::str::FromStr;

use cli::{Cli, Pattern, Vanity};

use miner::{mine_move_address, mine_resource_address};
use num_cpus;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let cli = Cli::parse();

	match cli.command {
		Vanity::Move { starts, ends } => {
			let starts_str = if let Some(s) = starts {
				Pattern::from_str(&s)?;
				s
			} else {
				String::new()
			};
			let ends_str = if let Some(s) = ends {
				Pattern::from_str(&s)?;
				s
			} else {
				String::new()
			};

			let account = mine_move_address(&starts_str, &ends_str, num_cpus::get());

			println!("Found Move address: {}", account.address());
			println!("Private key (hex): {}", hex::encode(account.private_key().to_bytes()));
		}
		Vanity::Resource { address, starts, ends } => {
			let address = match address {
				Some(ref a) => AccountAddress::from_str(a)?,
				None => return Err("Move address is required".into()),
			};

			let starts_str = if let Some(s) = starts {
				Pattern::from_str(&s)?;
				s
			} else {
				String::new()
			};
			let ends_str = if let Some(s) = ends {
				Pattern::from_str(&s)?;
				s
			} else {
				String::new()
			};

			let (resource_address, seed) =
				mine_resource_address(&address, &starts_str, &ends_str, num_cpus::get());

			println!("Found Resource Address: {}", resource_address);
			println!("Seed: {}", hex::encode(seed));
		}
	}

	Ok(())
}
