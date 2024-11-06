use crate::models::*;
use crate::schema::*;
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
	pub locked_events: Vec<LockedEvent>,
	pub initiator_completed_events: Vec<InitiatorCompletedEvent>,
	pub counter_part_completed_events: Vec<CounterPartCompletedEvent>,
	pub cancelled_events: Vec<CancelledEvent>,
	pub refunded_events: Vec<RefundedEvent>,
}

impl Client {
	/// Creates a new client with the given connection.
	pub fn new(conn: PgConnection) -> Self {
		Self { conn }
	}

	/// Gets the client from an environment variable containing the postgresql url.
	pub fn from_env() -> Result<Self, anyhow::Error> {
		let url = std::env::var("BRIDGE_INDEXER_DATABASE_URL").expect("DATABASE_URL must be set");
		let conn = PgConnection::establish(&url)
			.map_err(|e| anyhow::anyhow!("Failed to connect to postgresql instance: {}", e))?;
		Ok(Self::new(conn))
	}

	/// Inserts a new transfer action into the database.
	pub fn insert_transfer_action(
		&mut self,
		action_type: TransferActionType,
	) -> Result<(), diesel::result::Error> {
		match action_type {
			TransferActionType::LockBridgeTransfer {
				bridge_transfer_id,
				hash_lock,
				initiator,
				recipient,
				amount,
			} => {
				diesel::insert_into(lock_bridge_transfers::table)
					.values(NewLockBridgeTransfer {
						bridge_transfer_id: bridge_transfer_id.0.to_vec(),
						hash_lock: hash_lock.0.to_vec(),
						initiator: initiator.0.to_vec(),
						recipient: recipient.0.to_vec(),
						amount: amount.0.into(),
					})
					.execute(&mut self.conn)?;
			}
			TransferActionType::WaitAndCompleteInitiator(wait_time_secs, hash_lock_pre_image) => {
				diesel::insert_into(wait_and_complete_initiators::table)
					.values(NewWaitAndCompleteInitiator {
						wait_time_secs: wait_time_secs as i64,
						pre_image: hash_lock_pre_image.0.to_vec(),
					})
					.execute(&mut self.conn)?;
			}
			TransferActionType::RefundInitiator => {
				// do nothing
			}
			TransferActionType::TransferDone => {
				// do nothing
			}
			TransferActionType::NoAction => {
				// do nothing
			}
		}

		Ok(())
	}

	/// Gets the lock bridge transfer action with a given bridge transfer id.
	pub fn get_lock_bridge_transfer_action(
		&mut self,
		bridge_transfer_id: BridgeTransferId,
	) -> Result<LockBridgeTransfer, diesel::result::Error> {
		let bridge_transfer_id = bridge_transfer_id.0.to_vec();
		lock_bridge_transfers::table
			.filter(lock_bridge_transfers::bridge_transfer_id.eq(bridge_transfer_id))
			.first::<LockBridgeTransfer>(&mut self.conn)
	}

	/// Inserts a new bridge contract event into the database.
	pub fn insert_bridge_contract_event<A>(
		&mut self,
		contract_event: BridgeContractEvent<A>,
	) -> Result<(), diesel::result::Error>
	where
		A: Into<Vec<u8>>,
	{
		match contract_event {
			BridgeContractEvent::Initiated(bridge_transfer_details) => {
				diesel::insert_into(initiated_events::table)
					.values(NewInitiatedEvent {
						bridge_transfer_id: bridge_transfer_details.bridge_transfer_id.0.to_vec(),
						initiator_address: bridge_transfer_details.initiator_address.0.into(),
						recipient_address: bridge_transfer_details.recipient_address.0.to_vec(),
						hash_lock: bridge_transfer_details.hash_lock.0.to_vec(),
						time_lock: bridge_transfer_details.time_lock.0 as i64,
						amount: bridge_transfer_details.amount.0.into(),
						state: 0,
					})
					.execute(&mut self.conn)?;
			}
			BridgeContractEvent::Locked(lock_details) => {
				diesel::insert_into(locked_events::table)
					.values(NewLockedEvent {
						bridge_transfer_id: lock_details.bridge_transfer_id.0.to_vec(),
						initiator: lock_details.initiator.0.into(),
						recipient: lock_details.recipient.0.into(),
						hash_lock: lock_details.hash_lock.0.to_vec(),
						time_lock: lock_details.time_lock.0 as i64,
						amount: lock_details.amount.0.into(),
					})
					.execute(&mut self.conn)?;
			}
			BridgeContractEvent::InitialtorCompleted(initiator_completed_events) => {
				diesel::insert_into(initiator_completed_events::table)
					.values(NewInitiatorCompletedEvent {
						bridge_transfer_id: initiator_completed_events.0.to_vec(),
					})
					.execute(&mut self.conn)?;
			}
			BridgeContractEvent::CounterPartCompleted(bridge_transfer_id, hash_lock_pre_image) => {
				diesel::insert_into(counter_part_completed_events::table)
					.values(NewCounterPartCompletedEvent {
						bridge_transfer_id: bridge_transfer_id.0.to_vec(),
						pre_image: hash_lock_pre_image.0.to_vec(),
					})
					.execute(&mut self.conn)?;
			}
			BridgeContractEvent::Cancelled(bridge_transfer_id) => {
				diesel::insert_into(cancelled_events::table)
					.values(NewCancelledEvent { bridge_transfer_id: bridge_transfer_id.0.to_vec() })
					.execute(&mut self.conn)?;
			}
			BridgeContractEvent::Refunded(bridge_transfer_id) => {
				diesel::insert_into(refunded_events::table)
					.values(NewRefundedEvent { bridge_transfer_id: bridge_transfer_id.0.to_vec() })
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
		let bridge_transfer_id = bridge_transfer_id.0.to_vec();

		let initiated_events = initiated_events::table
			.filter(initiated_events::bridge_transfer_id.eq(bridge_transfer_id.clone()))
			.load::<InitiatedEvent>(&mut self.conn)?;

		let locked_events = locked_events::table
			.filter(locked_events::bridge_transfer_id.eq(bridge_transfer_id.clone()))
			.load::<LockedEvent>(&mut self.conn)?;

		let initiator_completed_events = initiator_completed_events::table
			.filter(initiator_completed_events::bridge_transfer_id.eq(bridge_transfer_id.clone()))
			.load::<InitiatorCompletedEvent>(&mut self.conn)?;

		let counter_part_completed_events = counter_part_completed_events::table
			.filter(
				counter_part_completed_events::bridge_transfer_id.eq(bridge_transfer_id.clone()),
			)
			.load::<CounterPartCompletedEvent>(&mut self.conn)?;

		let cancelled_events = cancelled_events::table
			.filter(cancelled_events::bridge_transfer_id.eq(bridge_transfer_id.clone()))
			.load::<CancelledEvent>(&mut self.conn)?;

		let refunded_events = refunded_events::table
			.filter(refunded_events::bridge_transfer_id.eq(bridge_transfer_id.clone()))
			.load::<RefundedEvent>(&mut self.conn)?;

		Ok(BridgeEventPackage {
			initiated_events,
			locked_events,
			initiator_completed_events,
			counter_part_completed_events,
			cancelled_events,
			refunded_events,
		})
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
