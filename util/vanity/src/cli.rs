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