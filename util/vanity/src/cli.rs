use std::{ops::Deref, str::FromStr};

/// Pattern.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) struct Pattern(Box<str>);

impl Deref for Pattern {
	type Target = str;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl Pattern {
	pub(super) fn into_bytes(self) -> Result<Vec<u8>, hex::FromHexError> {
		let mut string = self.to_string();

		if self.len() % 2 != 0 {
			string += "0"
		};

		hex::decode(string)
	}
}

/// Pattern errors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub(super) enum PatternError {
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

#[derive(Clone, Debug, clap::Parser)]
#[command(name = "vanity", about = "Vanity is a fast vanity address miner.")]
pub(super) enum Vanity {
	Move {
        #[clap(long)]
		starts_pattern: Option<String>,
        #[clap(long)]
		ends_pattern: Option<String>,
	},
}


#[cfg(test)]
mod tests {
	use super::*;
	use std::str::FromStr;
	use clap::Parser; // <- THIS is what you're missing!
	use assert_cmd::Command;
	use predicates::prelude::*;

	#[test]
    fn test_valid_pattern() {
        let p = Pattern::from_str("abcd").unwrap();
        assert_eq!(&*p, "abcd");
    }

    #[test]
	fn test_cli_parsing_starts_pattern() {
		let cli = Vanity::parse_from([
			"vanity",
			"move",
			"--starts-pattern",
			"abcd",
		]);
		match cli {
			Vanity::Move { starts_pattern, ends_pattern } => {
				assert_eq!(starts_pattern, Some("abcd".to_string()));
				assert_eq!(ends_pattern, None);
			}
		}
	}

    #[test]
fn test_cli_runs_with_starts() {
	let mut cmd = Command::cargo_bin("vanity").unwrap();
	let assert = cmd.args(&["move", "--starts-pattern", "de"]).assert().success();

	let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
	let address_line = output.lines().find(|l| l.contains("Found Move address:")).unwrap();
	let address = address_line.split(':').nth(1).unwrap().trim().trim_start_matches("0x");

	assert!(address.starts_with("de"), "Address does not start with 'de': {}", address);
}

#[test]
fn test_cli_runs_with_ends() {
	let mut cmd = Command::cargo_bin("vanity").unwrap();
	let assert = cmd.args(&["move", "--ends-pattern", "f0"]).assert().success();

	let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
	let address_line = output.lines().find(|l| l.contains("Found Move address:")).unwrap();
	let address = address_line.split(':').nth(1).unwrap().trim().trim_start_matches("0x");

	assert!(address.ends_with("f0"), "Address does not end with 'f0': {}", address);
}

#[test]
fn test_cli_runs_with_starts_and_ends() {
	let mut cmd = Command::cargo_bin("vanity").unwrap();
	let assert = cmd
		.args(&["move", "--starts-pattern", "de", "--ends-pattern", "f0"])
		.assert()
		.success();

	let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
	let address_line = output.lines().find(|l| l.contains("Found Move address:")).unwrap();
	let address = address_line.split(':').nth(1).unwrap().trim().trim_start_matches("0x");

	assert!(address.starts_with("de"), "Address does not start with 'de': {}", address);
	assert!(address.ends_with("f0"), "Address does not end with 'f0': {}", address);
}

}