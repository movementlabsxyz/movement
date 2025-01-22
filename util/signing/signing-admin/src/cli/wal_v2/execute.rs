use super::log::{
	KeyRotationMessage, Operation, RecvAppUpdateKey, RecvHsmUpdateKey, SendAppUpdateKey,
	SendHsmUpdateKey, StartRotateKey, TransactionId, Wal, WalEntry,
};

pub struct WalExecutor {}

impl WalExecutor {
	pub fn new() -> Self {
		Self {}
	}

	async fn execute_inner(&self, wal: Wal) -> Result<Wal, (Wal, anyhow::Error)> {
		Ok(wal.execute().await?)
	}

	pub async fn append(
		&self,
		wal: Wal,
		key_id: String,
		alias: String,
	) -> Result<Wal, (Wal, anyhow::Error)> {
		// form the message types for each step, place start rotate key in committed and the rest in prepared
		let start_rotate_key = StartRotateKey::new(key_id.clone(), alias.clone());
		let send_app_update_key = SendAppUpdateKey::new(key_id.clone(), alias.clone());
		let recv_app_update_key = RecvAppUpdateKey::new(key_id.clone(), alias.clone());
		let send_hsm_update_key = SendHsmUpdateKey::new(key_id.clone(), alias.clone());
		let recv_hsm_update_key = RecvHsmUpdateKey::new(key_id.clone(), alias.clone());

		// get a uuid string for the transaction id
		let transaction_id = uuid::Uuid::new_v4().to_string();

		// execute each step
		WalEntry::new(
			TransactionId(transaction_id.clone()),
			Operation::RotateKey,
			vec![
				KeyRotationMessage::SendAppUpdateKey(send_app_update_key),
				KeyRotationMessage::RecvAppUpdateKey(recv_app_update_key),
				KeyRotationMessage::SendHsmUpdateKey(send_hsm_update_key),
				KeyRotationMessage::RecvHsmUpdateKey(recv_hsm_update_key),
			],
			vec![KeyRotationMessage::StartRotateKey(start_rotate_key)],
		);

		Ok(wal)
	}

	/// Executes by appending to the Wal and then using the Wal executor.
	///
	/// Note: this presumes that the program will not crash before the successful Wal or the errant Wal is returned.
	/// That is, the persistence of the Wal to the disk is handled by the caller. This can be reimplemented as seen fit.
	pub async fn execute(
		&self,
		wal: Wal,
		key_id: String,
		alias: String,
	) -> Result<Wal, (Wal, anyhow::Error)> {
		// if the wal contains any entries, error out
		if wal.entries.len() > 0 {
			return Err((
				wal,
				anyhow::anyhow!("wal already contains entries a transaction did not complete"),
			));
		}

		// append the new entry
		let wal = self.append(wal, key_id, alias).await?;

		// execute the wal
		self.execute_inner(wal).await
	}

	/// Recovers by executing the wal
	pub async fn recover(&self, wal: Wal) -> Result<Wal, (Wal, anyhow::Error)> {
		// just execute the wal
		self.execute_inner(wal).await
	}
}
