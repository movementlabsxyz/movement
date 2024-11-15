use crate::schema::*;
use bigdecimal::BigDecimal;
use diesel::prelude::*;

// LockBridgeTransfer mapping
#[derive(Debug, Insertable, Default)]
#[diesel(table_name = lock_bridge_transfers)]
pub struct NewLockBridgeTransfer {
	pub bridge_transfer_id: String,
	pub hash_lock: String,
	pub initiator: String,
	pub recipient: String,
	pub amount: BigDecimal,
	pub created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Queryable, Insertable)]
#[diesel(table_name = lock_bridge_transfers)]
pub struct LockBridgeTransfer {
	pub id: i32,
	pub bridge_transfer_id: String,
	pub hash_lock: String,
	pub initiator: String,
	pub recipient: String,
	pub amount: BigDecimal,
	pub created_at: chrono::NaiveDateTime,
}

// WaitAndCompleteInitiator mapping
#[derive(Debug, Insertable, Default)]
#[diesel(table_name = wait_and_complete_initiators)]
pub struct NewWaitAndCompleteInitiator {
	pub wait_time_secs: i64,
	pub pre_image: String,
	pub created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Queryable, Insertable)]
#[diesel(table_name = wait_and_complete_initiators)]
pub struct WaitAndCompleteInitiator {
	pub id: i32,
	pub wait_time_secs: i64,
	pub pre_image: String,
	pub created_at: chrono::NaiveDateTime,
}

// InitiatedEvent mapping
#[derive(Debug, Insertable, Default)]
#[diesel(table_name = initiated_events)]
pub struct NewInitiatedEvent {
	pub bridge_transfer_id: String,
	pub initiator: String,
	pub recipient: String,
	pub hash_lock: String,
	pub time_lock: i64,
	pub amount: BigDecimal,
	pub state: i16,
	pub created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Queryable, Insertable)]
#[diesel(table_name = initiated_events)]
pub struct InitiatedEvent {
	pub id: i32,
	pub bridge_transfer_id: String,
	pub initiator: String,
	pub recipient: String,
	pub hash_lock: String,
	pub time_lock: i64,
	pub amount: BigDecimal,
	pub state: i16,
	pub created_at: chrono::NaiveDateTime,
}

// LockedEvent mapping
#[derive(Debug, Insertable, Default)]
#[diesel(table_name = locked_events)]
pub struct NewLockedEvent {
	pub bridge_transfer_id: String,
	pub initiator: String,
	pub recipient: String,
	pub hash_lock: String,
	pub time_lock: i64,
	pub amount: BigDecimal,
	pub created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Queryable, Insertable)]
#[diesel(table_name = locked_events)]
pub struct LockedEvent {
	pub id: i32,
	pub bridge_transfer_id: String,
	pub initiator: String,
	pub recipient: String,
	pub hash_lock: String,
	pub time_lock: i64,
	pub amount: BigDecimal,
	pub created_at: chrono::NaiveDateTime,
}

// InitiatorCompletedEvent mapping
#[derive(Debug, Insertable, Default)]
#[diesel(table_name = initiator_completed_events)]
pub struct NewInitiatorCompletedEvent {
	pub bridge_transfer_id: String,
	pub created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Queryable, Insertable)]
#[diesel(table_name = initiator_completed_events)]
pub struct InitiatorCompletedEvent {
	pub id: i32,
	pub bridge_transfer_id: String,
	pub created_at: chrono::NaiveDateTime,
}

// CounterPartCompletedEvent mapping
#[derive(Debug, Insertable, Default)]
#[diesel(table_name = counter_party_completed_events)]
pub struct NewCounterPartyCompletedEvent {
	pub bridge_transfer_id: String,
	pub pre_image: String,
	pub created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Queryable, Insertable)]
#[diesel(table_name = counter_party_completed_events)]
pub struct CounterPartyCompletedEvent {
	pub id: i32,
	pub bridge_transfer_id: String,
	pub pre_image: String,
	pub created_at: chrono::NaiveDateTime,
}

// CancelledEvent mapping
#[derive(Debug, Insertable, Default)]
#[diesel(table_name = cancelled_events)]
pub struct NewCancelledEvent {
	pub bridge_transfer_id: String,
	pub created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Queryable, Insertable)]
#[diesel(table_name = cancelled_events)]
pub struct CancelledEvent {
	pub id: i32,
	pub bridge_transfer_id: String,
	pub created_at: chrono::NaiveDateTime,
}

// RefundedEvent mapping
#[derive(Debug, Insertable, Default)]
#[diesel(table_name = refunded_events)]
pub struct NewRefundedEvent {
	pub bridge_transfer_id: String,
	pub created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Queryable, Insertable)]
#[diesel(table_name = refunded_events)]
pub struct RefundedEvent {
	pub id: i32,
	pub bridge_transfer_id: String,
	pub created_at: chrono::NaiveDateTime,
}
