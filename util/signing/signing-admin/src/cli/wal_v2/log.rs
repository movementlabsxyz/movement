use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TransactionId(pub String);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Operation {
	RotateKey,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StartRotateKey {
	pub key_id: String,
	pub alias: String,
}

impl StartRotateKey {
	pub fn new(key_id: String, alias: String) -> Self {
		Self { key_id, alias }
	}

	pub async fn execute(&self) -> Result<SendAppUpdateKey, anyhow::Error> {
		unimplemented!()
	}
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SendAppUpdateKey {
	pub key_id: String,
	pub alias: String,
}

impl SendAppUpdateKey {
	pub fn new(key_id: String, alias: String) -> Self {
		Self { key_id, alias }
	}

	pub async fn execute(&self) -> Result<RecvAppUpdateKey, anyhow::Error> {
		unimplemented!()
	}
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RecvAppUpdateKey {
	pub key_id: String,
	pub alias: String,
}

impl RecvAppUpdateKey {
	pub fn new(key_id: String, alias: String) -> Self {
		Self { key_id, alias }
	}

	pub async fn execute(&self) -> Result<SendHsmUpdateKey, anyhow::Error> {
		unimplemented!()
	}
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SendHsmUpdateKey {
	pub key_id: String,
	pub alias: String,
}

impl SendHsmUpdateKey {
	pub fn new(key_id: String, alias: String) -> Self {
		Self { key_id, alias }
	}

	pub async fn execute(&self) -> Result<RecvHsmUpdateKey, anyhow::Error> {
		unimplemented!()
	}
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RecvHsmUpdateKey {
	pub key_id: String,
	pub alias: String,
}

impl RecvHsmUpdateKey {
	pub fn new(key_id: String, alias: String) -> Self {
		Self { key_id, alias }
	}

	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		unimplemented!()
	}
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum KeyRotationMessage {
	StartRotateKey(StartRotateKey),
	SendAppUpdateKey(SendAppUpdateKey),
	RecvAppUpdateKey(RecvAppUpdateKey),
	SendHsmUpdateKey(SendHsmUpdateKey),
	RecvHsmUpdateKey(RecvHsmUpdateKey),
}

impl KeyRotationMessage {
	pub async fn execute(&self) -> Result<Self, anyhow::Error> {
		match self {
			KeyRotationMessage::StartRotateKey(msg) => {
				msg.execute().await.map(KeyRotationMessage::SendAppUpdateKey)
			}
			KeyRotationMessage::SendAppUpdateKey(msg) => {
				msg.execute().await.map(KeyRotationMessage::RecvAppUpdateKey)
			}
			KeyRotationMessage::RecvAppUpdateKey(msg) => {
				msg.execute().await.map(KeyRotationMessage::SendHsmUpdateKey)
			}
			KeyRotationMessage::SendHsmUpdateKey(msg) => {
				msg.execute().await.map(KeyRotationMessage::RecvHsmUpdateKey)
			}
			KeyRotationMessage::RecvHsmUpdateKey(msg) => {
				msg.execute().await.map(|_| KeyRotationMessage::RecvHsmUpdateKey(msg.clone()))
			}
		}
	}
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WalEntry {
	pub transaction_id: TransactionId,
	pub operation: Operation,
	pub prepared_messages: Vec<KeyRotationMessage>,
	pub committed_messages: Vec<KeyRotationMessage>,
}

impl WalEntry {
	pub fn new(
		transaction_id: TransactionId,
		operation: Operation,
		prepared_messages: Vec<KeyRotationMessage>,
		committed_messages: Vec<KeyRotationMessage>,
	) -> Self {
		Self { transaction_id, operation, prepared_messages, committed_messages }
	}

	/// Execute the [WalEntry] by executing all the prepared messages and storing the results in the committed messages.
	pub async fn execute(&self) -> Result<Self, (WalEntry, anyhow::Error)> {
		let mut entry = self.clone();
		for message in &entry.prepared_messages {
			entry
				.committed_messages
				.push(message.execute().await.map_err(|e| (entry.clone(), e))?);
		}
		Ok(entry)
	}
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Wal {
	pub entries: Vec<WalEntry>,
}

impl Wal {
	pub fn new() -> Self {
		Self { entries: vec![] }
	}

	/// Execute the [Wal] by executing all the entries in order.
	pub async fn execute(&self) -> Result<Self, (Wal, anyhow::Error)> {
		let mut wal = self.clone();
		for entry in wal.entries.clone() {
			wal.entries.push(entry.execute().await.map_err(|(_, e)| (wal.clone(), e))?);
		}
		Ok(wal)
	}

	/// Append a new [WalEntry] to the [Wal].
	pub fn append(&mut self, entry: WalEntry) {
		self.entries.push(entry);
	}
}
