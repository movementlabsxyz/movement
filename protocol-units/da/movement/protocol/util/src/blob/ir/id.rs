use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Id(Vec<u8>);

/// The id for an Ir Blob
impl Id {
	pub fn new(id: Vec<u8>) -> Self {
		Id(id)
	}

	pub fn as_slice(&self) -> &[u8] {
		self.0.as_slice()
	}

	pub fn into_vec(self) -> Vec<u8> {
		self.0
	}
}

impl From<Vec<u8>> for Id {
	fn from(id: Vec<u8>) -> Self {
		Id(id)
	}
}
