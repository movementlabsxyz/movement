use clap::{Parser, Subcommand};
use std::{ops::Deref, str::FromStr};

/// Pattern type for hex string filtering.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pattern(Box<str>);

impl Deref for Pattern {
	type Target = str;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl Pattern {
	/// Return the raw hex string without trying to decode it.
	pub fn as_str(&self) -> &str {
		&self.0
	}
}

/// Pattern parsing errors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum PatternError {
	#[error("the pattern's length exceeds 39 characters or the pattern is empty")]
	InvalidPatternLength,
	#[error("the pattern is not in hexadecimal format")]
	NonHexPattern,
}

impl FromStr for Pattern {
	type Err = PatternError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		if s.len() >= 40 || s.is_empty() {
			return Err(PatternError::InvalidPatternLength);
		}
		if s.chars().any(|c| !c.is_ascii_hexdigit()) {
			return Err(PatternError::NonHexPattern);
		}
		Ok(Self(s.into()))
	}
}

/// CLI definition for the `vanity` tool.
#[derive(Parser, Debug)]
#[command(name = "vanity", about = "Vanity is a fast vanity address miner.")]
pub struct Cli {
	#[command(subcommand)]
	pub command: Vanity,
}

/// Supported CLI subcommands.
#[derive(Debug, Subcommand)]
pub enum Vanity {
	Move {
		#[clap(long)]
		starts: Option<String>,
		#[clap(long)]
		ends: Option<String>,
	},
	Resource {
		#[clap(long)]
		address: Option<String>,
		#[clap(long)]
		starts: Option<String>,
		#[clap(long)]
		ends: Option<String>,
	},
}

#[cfg(test)]
mod tests {
	use super::{Cli, Pattern, Vanity};
	use assert_cmd::Command;
	use clap::Parser;
	use std::str::FromStr;

	#[test]
	fn test_valid_pattern() {
		let p = Pattern::from_str("abcd").unwrap();
		assert_eq!(&*p, "abcd");
	}

	#[test]
	fn test_cli_parsing_starts() {
		let cli = Cli::parse_from(["vanity", "move", "--starts", "abcd"]);
		match cli.command {
			Vanity::Move { starts, ends } => {
				assert_eq!(starts, Some("abcd".to_string()));
				assert_eq!(ends, None);
			}
			_ => panic!("Expected Move variant"),
		}
	}

	#[test]
	fn test_resource_parsing_starts() {
		let cli = Cli::parse_from([
			"vanity",
			"resource",
			"--address",
			"0x5e04c2f5bf1a89d3431d2c047626acbba41c900a69b10e7c24e5b919802c531f",
			"--starts",
			"abcd",
		]);
		match cli.command {
			Vanity::Resource { address, starts, ends } => {
				assert_eq!(
					address,
					Some(
						"0x5e04c2f5bf1a89d3431d2c047626acbba41c900a69b10e7c24e5b919802c531f"
							.to_string()
					)
				);
				assert_eq!(starts, Some("abcd".to_string()));
				assert_eq!(ends, None);
			}
			_ => panic!("Expected Resource variant"),
		}
	}

	#[test]
	fn test_cli_runs_with_starts() {
		let mut cmd = Command::cargo_bin("vanity").unwrap();
		let assert = cmd.args(&["move", "--starts", "de"]).assert().success();

		let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
		let address_line = output.lines().find(|l| l.contains("Found Move address:")).unwrap();
		let address = address_line.split(':').nth(1).unwrap().trim().trim_start_matches("0x");

		assert!(address.starts_with("de"), "Address does not start with 'de': {}", address);
	}

	#[test]
	fn test_cli_runs_with_ends() {
		let mut cmd = Command::cargo_bin("vanity").unwrap();
		let assert = cmd.args(&["move", "--ends", "f0"]).assert().success();

		let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
		let address_line = output.lines().find(|l| l.contains("Found Move address:")).unwrap();
		let address = address_line.split(':').nth(1).unwrap().trim().trim_start_matches("0x");

		assert!(address.ends_with("f0"), "Address does not end with 'f0': {}", address);
	}

	#[test]
	fn test_cli_runs_with_starts_and_ends() {
		let mut cmd = Command::cargo_bin("vanity").unwrap();
		let assert = cmd.args(&["move", "--starts", "de", "--ends", "f0"]).assert().success();

		let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
		let address_line = output.lines().find(|l| l.contains("Found Move address:")).unwrap();
		let address = address_line.split(':').nth(1).unwrap().trim().trim_start_matches("0x");

		assert!(address.starts_with("de"), "Address does not start with 'de': {}", address);
		assert!(address.ends_with("f0"), "Address does not end with 'f0': {}", address);
	}

	#[test]
	fn test_resource_runs_with_starts_and_ends() {
		let mut cmd = Command::cargo_bin("vanity").unwrap();
		let assert = cmd
			.args(&[
				"resource",
				"--address",
				"0x5e04c2f5bf1a89d3431d2c047626acbba41c900a69b10e7c24e5b919802c531f",
				"--starts",
				"de",
				"--ends",
				"f0",
			])
			.assert()
			.success();

		let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
		let address_line = output.lines().find(|l| l.contains("Found Resource Address:")).unwrap();
		let address = address_line.split(':').nth(1).unwrap().trim().trim_start_matches("0x");

		assert!(address.starts_with("de"), "Address does not start with 'de': {}", address);
		assert!(address.ends_with("f0"), "Address does not end with 'f0': {}", address);
	}
}
