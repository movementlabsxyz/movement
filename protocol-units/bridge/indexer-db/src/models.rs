use crate::schema::*;
use bigdecimal::BigDecimal;
use diesel::prelude::*;

// InitiatedEvent mapping
#[derive(Debug, Insertable, Default)]
#[diesel(table_name = initiated_events)]
pub struct NewInitiatedEvent {
	pub bridge_transfer_id: String,
	pub initiator: String,
	pub recipient: String,
	pub amount: BigDecimal,
	pub nonce: BigDecimal,
	pub created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Queryable, Insertable)]
#[diesel(table_name = initiated_events)]
pub struct InitiatedEvent {
	pub id: i32,
	pub bridge_transfer_id: String,
	pub initiator: String,
	pub recipient: String,
	pub amount: BigDecimal,
	pub nonce: BigDecimal,
	pub created_at: chrono::NaiveDateTime,
}

// LockedEvent mapping
#[derive(Debug, Insertable, Default)]
#[diesel(table_name = completed_events)]
pub struct NewCompletedEvent {
	pub bridge_transfer_id: String,
	pub initiator: String,
	pub recipient: String,
	pub amount: BigDecimal,
	pub nonce: BigDecimal,
	pub created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Queryable, Insertable)]
#[diesel(table_name = completed_events)]
pub struct CompletedEvent {
	pub id: i32,
	pub bridge_transfer_id: String,
	pub initiator: String,
	pub recipient: String,
	pub amount: BigDecimal,
	pub nonce: BigDecimal,
	pub created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Insertable, Default)]
#[diesel(table_name = complete_bridge_transfers)]
pub struct CompleteBridgeTransferAction {
	pub bridge_transfer_id: String,
	pub initiator: String,
	pub recipient: String,
	pub amount: BigDecimal,
	pub nonce: BigDecimal,
	pub created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Insertable, Default)]
#[diesel(table_name = completed_remove_state)]
pub struct CompletedRemoveStateAction {
	pub bridge_transfer_id: String,
	pub created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Insertable, Default)]
#[diesel(table_name = abort_replay_transfers)]
pub struct AbortReplayTransferAction {
	pub bridge_transfer_id: String,
	pub initiator: String,
	pub recipient: String,
	pub amount: BigDecimal,
	pub nonce: BigDecimal,
	pub wait_time_sec: BigDecimal,
	pub created_at: chrono::NaiveDateTime,
}
