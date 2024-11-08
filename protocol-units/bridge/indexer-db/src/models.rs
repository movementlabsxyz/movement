use crate::schema::*;
use bigdecimal::BigDecimal;
use diesel::prelude::*;

// LockBridgeTransfer mapping
#[derive(Debug, Insertable)]
#[table_name = "lock_bridge_transfers"]
pub struct NewLockBridgeTransfer {
	pub bridge_transfer_id: String,
	pub hash_lock: String,
	pub initiator: String,
	pub recipient: String,
	pub amount: BigDecimal,
}

#[derive(Debug, Queryable, Insertable)]
#[table_name = "lock_bridge_transfers"]
pub struct LockBridgeTransfer {
	pub id: i32,
	pub bridge_transfer_id: String,
	pub hash_lock: String,
	pub initiator: String,
	pub recipient: String,
	pub amount: BigDecimal,
}

// WaitAndCompleteInitiator mapping
#[derive(Debug, Insertable)]
#[table_name = "wait_and_complete_initiators"]
pub struct NewWaitAndCompleteInitiator {
	pub wait_time_secs: i64,
	pub pre_image: String,
}

#[derive(Debug, Queryable, Insertable)]
#[table_name = "wait_and_complete_initiators"]
pub struct WaitAndCompleteInitiator {
	pub id: i32,
	pub wait_time_secs: i64,
	pub pre_image: String,
}

// InitiatedEvent mapping
#[derive(Debug, Insertable)]
#[table_name = "initiated_events"]
pub struct NewInitiatedEvent {
	pub bridge_transfer_id: String,
	pub initiator: String,
	pub recipient: String,
	pub hash_lock: String,
	pub time_lock: i64,
	pub amount: BigDecimal,
	pub state: i16,
}

#[derive(Debug, Queryable, Insertable)]
#[table_name = "initiated_events"]
pub struct InitiatedEvent {
	pub id: i32,
	pub bridge_transfer_id: String,
	pub initiator: String,
	pub recipient: String,
	pub hash_lock: String,
	pub time_lock: i64,
	pub amount: BigDecimal,
	pub state: i16,
}

// LockedEvent mapping
#[derive(Debug, Insertable)]
#[table_name = "locked_events"]
pub struct NewLockedEvent {
	pub bridge_transfer_id: String,
	pub initiator: String,
	pub recipient: String,
	pub hash_lock: String,
	pub time_lock: i64,
	pub amount: BigDecimal,
}

#[derive(Debug, Queryable, Insertable)]
#[table_name = "locked_events"]
pub struct LockedEvent {
	pub id: i32,
	pub bridge_transfer_id: String,
	pub initiator: String,
	pub recipient: String,
	pub hash_lock: String,
	pub time_lock: i64,
	pub amount: BigDecimal,
}

// InitiatorCompletedEvent mapping
#[derive(Debug, Insertable)]
#[table_name = "initiator_completed_events"]
pub struct NewInitiatorCompletedEvent {
	pub bridge_transfer_id: String,
}

#[derive(Debug, Queryable, Insertable)]
#[table_name = "initiator_completed_events"]
pub struct InitiatorCompletedEvent {
	pub id: i32,
	pub bridge_transfer_id: String,
}

// CounterPartyCompletedEvent mapping
#[derive(Debug, Insertable)]
#[table_name = "counter_party_completed_events"]
pub struct NewCounterPartyCompletedEvent {
	pub bridge_transfer_id: String,
	pub pre_image: String,
}

#[derive(Debug, Queryable, Insertable)]
#[table_name = "counter_party_completed_events"]
pub struct CounterPartyCompletedEvent {
	pub id: i32,
	pub bridge_transfer_id: String,
	pub pre_image: String,
}

// CancelledEvent mapping
#[derive(Debug, Insertable)]
#[table_name = "cancelled_events"]
pub struct NewCancelledEvent {
	pub bridge_transfer_id: String,
}

#[derive(Debug, Queryable, Insertable)]
#[table_name = "cancelled_events"]
pub struct CancelledEvent {
	pub id: i32,
	pub bridge_transfer_id: String,
}

// RefundedEvent mapping
#[derive(Debug, Insertable)]
#[table_name = "refunded_events"]
pub struct NewRefundedEvent {
	pub bridge_transfer_id: String,
}

#[derive(Debug, Queryable, Insertable)]
#[table_name = "refunded_events"]
pub struct RefundedEvent {
	pub id: i32,
	pub bridge_transfer_id: String,
}
