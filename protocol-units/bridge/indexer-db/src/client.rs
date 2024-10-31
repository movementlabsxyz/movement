use crate::models::*;
use crate::schema::*;
use bridge_util::chains::bridge_contracts::BridgeContractEvent;
use bridge_util::TransferActionType;
use diesel::pg::PgConnection;
use diesel::prelude::*;

pub struct Client {
	conn: PgConnection,
}

impl Client {
	/// Creates a new client with the given connection.
	pub fn new(conn: PgConnection) -> Self {
		Self { conn }
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
					.values(LockBridgeTransfer {
						bridge_transfer_id: bridge_transfer_id.0.to_vec(),
						hash_lock: hash_lock.0.to_vec(),
						initiator: initiator.0.to_vec(),
						recipient: recipient.0.to_vec(),
						amount: amount.value().into(),
					})
					.execute(&mut self.conn)?;
			}
			TransferActionType::WaitAndCompleteInitiator(wait_time_secs, hash_lock_pre_image) => {
				diesel::insert_into(wait_and_complete_initiators::table)
					.values(WaitAndCompleteInitiator {
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
					.values(InitiatedEvent {
						bridge_transfer_id: bridge_transfer_details.bridge_transfer_id.0.to_vec(),
						initiator_address: bridge_transfer_details.initiator_address.0.into(),
						recipient_address: bridge_transfer_details.recipient_address.0.to_vec(),
						hash_lock: bridge_transfer_details.hash_lock.0.to_vec(),
						time_lock: bridge_transfer_details.time_lock.0 as i64,
						amount: bridge_transfer_details.amount.value().into(),
						state: 0,
					})
					.execute(&mut self.conn)?;
			}
			BridgeContractEvent::Locked(lock_details) => {
				diesel::insert_into(locked_events::table)
					.values(LockedEvent {
						bridge_transfer_id: lock_details.bridge_transfer_id.0.to_vec(),
						initiator: lock_details.initiator.0.into(),
						recipient: lock_details.recipient.0.into(),
						hash_lock: lock_details.hash_lock.0.to_vec(),
						time_lock: lock_details.time_lock.0 as i64,
						amount: lock_details.amount.value().into(),
					})
					.execute(&mut self.conn)?;
			}
			BridgeContractEvent::InitialtorCompleted(initiator_completed_events) => {
				diesel::insert_into(initiator_completed_events::table)
					.values(InitiatorCompletedEvent {
						bridge_transfer_id: initiator_completed_events.0.to_vec(),
					})
					.execute(&mut self.conn)?;
			}
			BridgeContractEvent::CounterPartCompleted(bridge_transfer_id, hash_lock_pre_image) => {
				diesel::insert_into(counter_part_completed_events::table)
					.values(CounterPartCompletedEvent {
						bridge_transfer_id: bridge_transfer_id.0.to_vec(),
						pre_image: hash_lock_pre_image.0.to_vec(),
					})
					.execute(&mut self.conn)?;
			}
			BridgeContractEvent::Cancelled(bridge_transfer_id) => {
				diesel::insert_into(cancelled_events::table)
					.values(CancelledEvent { bridge_transfer_id: bridge_transfer_id.0.to_vec() })
					.execute(&mut self.conn)?;
			}
			BridgeContractEvent::Refunded(bridge_transfer_id) => {
				diesel::insert_into(refunded_events::table)
					.values(RefundedEvent { bridge_transfer_id: bridge_transfer_id.0.to_vec() })
					.execute(&mut self.conn)?;
			}
		}

		Ok(())
	}
}
