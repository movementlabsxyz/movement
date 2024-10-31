use crate::schema::*;
use bigdecimal::BigDecimal;
use diesel::prelude::*;

// LockBridgeTransfer mapping
#[derive(Debug, Insertable)]
#[table_name = "lock_bridge_transfers"]
pub struct NewLockBridgeTransfer {
	pub bridge_transfer_id: Vec<u8>,
	pub hash_lock: Vec<u8>,
	pub initiator: Vec<u8>,
	pub recipient: Vec<u8>,
	pub amount: BigDecimal,
}

#[derive(Debug, Queryable, Insertable)]
#[table_name = "lock_bridge_transfers"]
pub struct LockBridgeTransfer {
	pub id: i32,
	pub bridge_transfer_id: Vec<u8>,
	pub hash_lock: Vec<u8>,
	pub initiator: Vec<u8>,
	pub recipient: Vec<u8>,
	pub amount: BigDecimal,
}

// WaitAndCompleteInitiator mapping
#[derive(Debug, Insertable)]
#[table_name = "wait_and_complete_initiators"]
pub struct NewWaitAndCompleteInitiator {
	pub wait_time_secs: i64,
	pub pre_image: Vec<u8>,
}

#[derive(Debug, Queryable, Insertable)]
#[table_name = "wait_and_complete_initiators"]
pub struct WaitAndCompleteInitiator {
	pub id: i32,
	pub wait_time_secs: i64,
	pub pre_image: Vec<u8>,
}

// InitiatedEvent mapping
#[derive(Debug, Insertable)]
#[table_name = "initiated_events"]
pub struct NewInitiatedEvent {
	pub bridge_transfer_id: Vec<u8>,
	pub initiator_address: Vec<u8>,
	pub recipient_address: Vec<u8>,
	pub hash_lock: Vec<u8>,
	pub time_lock: i64,
	pub amount: BigDecimal,
	pub state: i16,
}

#[derive(Debug, Queryable, Insertable)]
#[table_name = "initiated_events"]
pub struct InitiatedEvent {
	pub id: i32,
	pub bridge_transfer_id: Vec<u8>,
	pub initiator_address: Vec<u8>,
	pub recipient_address: Vec<u8>,
	pub hash_lock: Vec<u8>,
	pub time_lock: i64,
	pub amount: BigDecimal,
	pub state: i16,
}

// LockedEvent mapping
#[derive(Debug, Insertable)]
#[table_name = "locked_events"]
pub struct NewLockedEvent {
	pub bridge_transfer_id: Vec<u8>,
	pub initiator: Vec<u8>,
	pub recipient: Vec<u8>,
	pub hash_lock: Vec<u8>,
	pub time_lock: i64,
	pub amount: BigDecimal,
}

#[derive(Debug, Queryable, Insertable)]
#[table_name = "locked_events"]
pub struct LockedEvent {
	pub id: i32,
	pub bridge_transfer_id: Vec<u8>,
	pub initiator: Vec<u8>,
	pub recipient: Vec<u8>,
	pub hash_lock: Vec<u8>,
	pub time_lock: i64,
	pub amount: BigDecimal,
}

// InitiatorCompletedEvent mapping
#[derive(Debug, Insertable)]
#[table_name = "initiator_completed_events"]
pub struct NewInitiatorCompletedEvent {
	pub bridge_transfer_id: Vec<u8>,
}

#[derive(Debug, Queryable, Insertable)]
#[table_name = "initiator_completed_events"]
pub struct InitiatorCompletedEvent {
	pub id: i32,
	pub bridge_transfer_id: Vec<u8>,
}

// CounterPartCompletedEvent mapping
#[derive(Debug, Insertable)]
#[table_name = "counter_part_completed_events"]
pub struct NewCounterPartCompletedEvent {
	pub bridge_transfer_id: Vec<u8>,
	pub pre_image: Vec<u8>,
}

#[derive(Debug, Queryable, Insertable)]
#[table_name = "counter_part_completed_events"]
pub struct CounterPartCompletedEvent {
	pub id: i32,
	pub bridge_transfer_id: Vec<u8>,
	pub pre_image: Vec<u8>,
}

// CancelledEvent mapping
#[derive(Debug, Insertable)]
#[table_name = "cancelled_events"]
pub struct NewCancelledEvent {
	pub bridge_transfer_id: Vec<u8>,
}

#[derive(Debug, Queryable, Insertable)]
#[table_name = "cancelled_events"]
pub struct CancelledEvent {
	pub id: i32,
	pub bridge_transfer_id: Vec<u8>,
}

// RefundedEvent mapping
#[derive(Debug, Insertable)]
#[table_name = "refunded_events"]
pub struct NewRefundedEvent {
	pub bridge_transfer_id: Vec<u8>,
}

#[derive(Debug, Queryable, Insertable)]
#[table_name = "refunded_events"]
pub struct RefundedEvent {
	pub id: i32,
	pub bridge_transfer_id: Vec<u8>,
}
