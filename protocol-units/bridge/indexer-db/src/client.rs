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
		&self,
		action_type: TransferActionType,
	) -> Result<(), diesel::result::Error> {
		match action_type {
			TransferActionType::LockBridgeTransfer(_) => {
				diesel::insert_into(transfer_actions::table)
					.values(NewTransferAction { action_type })
					.execute(&self.conn)?;
			}
			TransferActionType::WaitAndCompleteInitiator(u64, HashLockPreImage) => {
				diesel::insert_into(transfer_actions::table)
					.values(NewTransferAction { action_type })
					.execute(&self.conn)?;
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
		&self,
		contract_event: BridgeContractEvent<A>,
	) -> Result<(), diesel::result::Error> {
		match contract_event {
			BridgeContractEvent::Initiated(_) => {
				diesel::insert_into(bridge_contract_events::table)
					.values(NewBridgeContractEvent { contract_event })
					.execute(&self.conn)?;
			}
			BridgeContractEvent::Locked(_) => {
				diesel::insert_into(bridge_contract_events::table)
					.values(NewBridgeContractEvent { contract_event })
					.execute(&self.conn)?;
			}
			BridgeContractEvent::InitialtorCompleted(_) => {
				diesel::insert_into(bridge_contract_events::table)
					.values(NewBridgeContractEvent { contract_event })
					.execute(&self.conn)?;
			}
			BridgeContractEvent::CounterPartCompleted(_, _) => {
				diesel::insert_into(bridge_contract_events::table)
					.values(NewBridgeContractEvent { contract_event })
					.execute(&self.conn)?;
			}
			BridgeContractEvent::Cancelled(_) => {
				diesel::insert_into(bridge_contract_events::table)
					.values(NewBridgeContractEvent { contract_event })
					.execute(&self.conn)?;
			}
			BridgeContractEvent::Refunded(_) => {
				diesel::insert_into(bridge_contract_events::table)
					.values(NewBridgeContractEvent { contract_event })
					.execute(&self.conn)?;
			}
		}

		Ok(())
	}
}
