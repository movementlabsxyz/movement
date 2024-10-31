use diesel::prelude::*;

table! {
	lock_bridge_transfers (id) {
		id -> Int4,
		bridge_transfer_id -> Binary,
		hash_lock -> Binary,
		initiator -> Binary,
		recipient -> Binary,
		amount -> Numeric,
	}
}

table! {
	wait_and_complete_initiators (id) {
		id -> Int4,
		timestamp -> BigInt,
		pre_image -> Binary,
	}
}

table! {
	initiated_events (id) {
		id -> Int4,
		bridge_transfer_id -> Binary,
		initiator_address -> Binary,
		recipient_address -> Binary,
		hash_lock -> Binary,
		time_lock -> BigInt,
		amount -> Numeric,
		state -> Int2,
	}
}

table! {
	locked_events (id) {
		id -> Int4,
		bridge_transfer_id -> Binary,
		initiator -> Binary,
		recipient -> Binary,
		hash_lock -> Binary,
		time_lock -> BigInt,
		amount -> Numeric,
	}
}

table! {
	initiator_completed_events (id) {
		id -> Int4,
		bridge_transfer_id -> Binary,
	}
}

table! {
	counter_part_completed_events (id) {
		id -> Int4,
		bridge_transfer_id -> Binary,
		pre_image -> Binary,
	}
}

table! {
	cancelled_events (id) {
		id -> Int4,
		bridge_transfer_id -> Binary,
	}
}

table! {
	refunded_events (id) {
		id -> Int4,
		bridge_transfer_id -> Binary,
	}
}
