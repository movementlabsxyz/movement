use crate::schema::*;
use bigdecimal::BigDecimal;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

// LockBridgeTransfer mapping
#[derive(Debug, Queryable, Insertable, Serialize, Deserialize)]
#[table_name = "lock_bridge_transfers"]
pub struct LockBridgeTransfer {
	pub bridge_transfer_id: Vec<u8>,
	pub hash_lock: Vec<u8>,
	pub initiator: Vec<u8>,
	pub recipient: Vec<u8>,
	pub amount: BigDecimal,
}

// WaitAndCompleteInitiator mapping
#[derive(Debug, Queryable, Insertable, Serialize, Deserialize)]
#[table_name = "wait_and_complete_initiators"]
pub struct WaitAndCompleteInitiator {
	pub wait_time_secs: i64,
	pub pre_image: Vec<u8>,
}

// InitiatedEvent mapping
#[derive(Debug, Queryable, Insertable, Serialize, Deserialize)]
#[table_name = "initiated_events"]
pub struct InitiatedEvent {
	pub bridge_transfer_id: Vec<u8>,
	pub initiator_address: Vec<u8>,
	pub recipient_address: Vec<u8>,
	pub hash_lock: Vec<u8>,
	pub time_lock: i64,
	pub amount: BigDecimal,
	pub state: i16,
}

// LockedEvent mapping
#[derive(Debug, Queryable, Insertable, Serialize, Deserialize)]
#[table_name = "locked_events"]
pub struct LockedEvent {
	pub bridge_transfer_id: Vec<u8>,
	pub initiator: Vec<u8>,
	pub recipient: Vec<u8>,
	pub hash_lock: Vec<u8>,
	pub time_lock: i64,
	pub amount: BigDecimal,
}

// InitiatorCompletedEvent mapping
#[derive(Debug, Queryable, Insertable, Serialize, Deserialize)]
#[table_name = "initiator_completed_events"]
pub struct InitiatorCompletedEvent {
	pub bridge_transfer_id: Vec<u8>,
}

// CounterPartCompletedEvent mapping
#[derive(Debug, Queryable, Insertable, Serialize, Deserialize)]
#[table_name = "counter_part_completed_events"]
pub struct CounterPartCompletedEvent {
	pub bridge_transfer_id: Vec<u8>,
	pub pre_image: Vec<u8>,
}

// CancelledEvent mapping
#[derive(Debug, Queryable, Insertable, Serialize, Deserialize)]
#[table_name = "cancelled_events"]
pub struct CancelledEvent {
	pub bridge_transfer_id: Vec<u8>,
}

// RefundedEvent mapping
#[derive(Debug, Queryable, Insertable, Serialize, Deserialize)]
#[table_name = "refunded_events"]
pub struct RefundedEvent {
	pub bridge_transfer_id: Vec<u8>,
}
