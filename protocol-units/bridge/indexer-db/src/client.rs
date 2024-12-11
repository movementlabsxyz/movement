use crate::migrations::run_migrations;
use crate::models::*;
use crate::schema::*;
use bridge_config::Config;
use bridge_util::chains::bridge_contracts::BridgeContractEvent;
use bridge_util::types::BridgeTransferId;
use bridge_util::TransferActionType;
use diesel::pg::PgConnection;
use diesel::prelude::*;

pub struct Client {
	conn: PgConnection,
}

pub struct BridgeEventPackage {
	pub initiated_events: Vec<InitiatedEvent>,
	pub completed_events: Vec<CompletedEvent>,
}

impl Client {
	/// Creates a new client with the given connection.
	pub fn new(conn: PgConnection) -> Self {
		Self { conn }
	}

	pub fn from_bridge_config(config: &Config) -> Result<Self, anyhow::Error> {
		let conn = PgConnection::establish(&config.indexer.indexer_url)
			.map_err(|e| anyhow::anyhow!("Failed to connect to postgresql instance: {}", e))?;
		Ok(Self::new(conn))
	}

	/// Run migrations on the database.
	pub fn run_migrations(&mut self) -> Result<(), anyhow::Error> {
		run_migrations(&mut self.conn)?;
		Ok(())
	}

	/// Inserts a new bridge contract event into the database.
	pub fn insert_bridge_contract_event<A>(
		&mut self,
		contract_event: BridgeContractEvent<A>,
	) -> Result<(), diesel::result::Error>
	where
		A: Into<Vec<u8>>,
	{
		tracing::info!("Indexer insert_bridge_contract_event event:{contract_event}");
		match contract_event {
			BridgeContractEvent::Initiated(details) => {
				diesel::insert_into(initiated_events::table)
					.values(NewInitiatedEvent {
						bridge_transfer_id: hex::encode(details.bridge_transfer_id.0.to_vec()),
						initiator: hex::encode(details.initiator.0.into()),
						recipient: hex::encode(details.recipient.0.to_vec()),
						amount: details.amount.0.into(),
						nonce: details.nonce.0.into(),
						created_at: chrono::Utc::now().naive_utc(),
					})
					.execute(&mut self.conn)?;
			}
			BridgeContractEvent::Completed(details) => {
				diesel::insert_into(completed_events::table)
					.values(NewCompletedEvent {
						bridge_transfer_id: hex::encode(details.bridge_transfer_id.0.to_vec()),
						initiator: hex::encode::<Vec<u8>>(details.initiator.0.into()),
						recipient: hex::encode::<Vec<u8>>(details.recipient.0.into()),
						amount: details.amount.0.into(),
						nonce: details.nonce.0.into(),
						created_at: chrono::Utc::now().naive_utc(),
					})
					.execute(&mut self.conn)?;
			}
		}

		Ok(())
	}

	/// Finds all events with a bridge transfer id.
	pub fn find_all_events_for_bridge_transfer_id(
		&mut self,
		bridge_transfer_id: BridgeTransferId,
	) -> Result<BridgeEventPackage, diesel::result::Error> {
		let bridge_transfer_id = hex::encode(bridge_transfer_id.0.to_vec());

		let initiated_events = initiated_events::table
			.filter(initiated_events::bridge_transfer_id.eq(bridge_transfer_id.clone()))
			.load::<InitiatedEvent>(&mut self.conn)?;

		let completed_events = completed_events::table
			.filter(completed_events::bridge_transfer_id.eq(bridge_transfer_id.clone()))
			.load::<CompletedEvent>(&mut self.conn)?;

		Ok(BridgeEventPackage { initiated_events, completed_events })
	}

	/// Inserts a new relayer action into the database.
	pub fn insert_relayer_actions(
		&mut self,
		bridge_transfer_id: BridgeTransferId,
		action_type: TransferActionType,
	) -> Result<(), diesel::result::Error> {
		match action_type {
			TransferActionType::CompleteBridgeTransfer {
				bridge_transfer_id,
				initiator,
				recipient,
				amount,
				nonce,
			} => {
				diesel::insert_into(complete_bridge_transfers::table)
					.values(CompleteBridgeTransferAction {
						bridge_transfer_id: hex::encode(bridge_transfer_id.0.to_vec()),
						initiator: hex::encode(initiator.0.to_vec()),
						recipient: hex::encode(recipient.0.to_vec()),
						amount: amount.0.into(),
						nonce: nonce.0.into(),
						created_at: chrono::Utc::now().naive_utc(),
					})
					.execute(&mut self.conn)?;
			}
			TransferActionType::CompletedRemoveState => {
				diesel::insert_into(completed_remove_state::table)
					.values(CompletedRemoveStateAction {
						bridge_transfer_id: hex::encode(bridge_transfer_id.0.to_vec()),
						created_at: chrono::Utc::now().naive_utc(),
					})
					.execute(&mut self.conn)?;
			}
			TransferActionType::AbortedReplay {
				bridge_transfer_id,
				initiator,
				recipient,
				amount,
				nonce,
				wait_time_sec,
			} => {
				diesel::insert_into(abort_replay_transfers::table)
					.values(AbortReplayTransferAction {
						bridge_transfer_id: hex::encode(bridge_transfer_id.0.to_vec()),
						initiator: hex::encode(initiator.0.to_vec()),
						recipient: hex::encode(recipient.0.to_vec()),
						amount: amount.0.into(),
						nonce: nonce.0.into(),
						wait_time_sec: wait_time_sec.into(),
						created_at: chrono::Utc::now().naive_utc(),
					})
					.execute(&mut self.conn)?;
			}
			TransferActionType::NoAction => (),
		}

		Ok(())
	}
}

/*#[cfg(test)]
pub mod test {
	use super::*;
	use crate::migrations::MIGRATIONS;
	use bridge_util::types::Amount;
	use bridge_util::types::{BridgeAddress, BridgeTransferId, HashLock};
	use bridge_util::TransferActionType;
	use diesel::connection::Connection;
	use diesel::pg::Pg;
	use diesel::sqlite::SqliteConnection;
	use diesel_migrations::MigrationHarness;

	#[tokio::test]
	async fn test_insert_transfer_action() -> Result<(), anyhow::Error> {
		// embed the postgresql instance
		let mut postgresql = PostgreSQL::default();
		postgresql.setup().await?;
		postgresql.start().await?;

		// create a connection to the postgresql instance
		let uri = postgresql.settings().url();

		// connect to the pg instance
		let mut conn = PgConnection::establish(&uri)
			.map_err(|e| anyhow::anyhow!("Failed to connect to postgresql instance: {}", e))?;

		conn.run_pending_migrations(MIGRATIONS).map_err(|e| {
			anyhow::anyhow!("Failed to run migrations for bridge indexer db: {}", e)
		})?;

		// create the client
		let mut client = Client::new(conn);

		// insert a transfer action
		let action_type = TransferActionType::LockBridgeTransfer {
			bridge_transfer_id: BridgeTransferId::test(),
			hash_lock: HashLock::test(),
			initiator: BridgeAddress::<Vec<u8>>::test(),
			recipient: BridgeAddress::<Vec<u8>>::test(),
			amount: Amount::test(),
		};
		client.insert_transfer_action(action_type)?;

		// get the transfer action
		let lock_bridge_transfer =
			client.get_lock_bridge_transfer_action(BridgeTransferId::test())?;

		// check the transfer action
		assert_eq!(lock_bridge_transfer.bridge_transfer_id, BridgeTransferId::test().0.to_vec());
		assert_eq!(lock_bridge_transfer.hash_lock, HashLock::test().0.to_vec());
		assert_eq!(lock_bridge_transfer.initiator, BridgeAddress::<Vec<u8>>::test().0.to_vec());
		assert_eq!(lock_bridge_transfer.recipient, BridgeAddress::<Vec<u8>>::test().0.to_vec());
		assert_eq!(lock_bridge_transfer.amount, Amount::test().value().into());

		Ok(())
	}
}*/
