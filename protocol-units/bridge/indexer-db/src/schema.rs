use diesel::prelude::*;

table! {
	lock_bridge_transfers (id) {
		id -> Int4,
		bridge_transfer_id -> Text,
		hash_lock -> Text,
		initiator -> Text,
		recipient -> Text,
		amount -> Numeric,
	}
}

table! {
	wait_and_complete_initiators (id) {
		id -> Int4,
		wait_time_secs -> BigInt,
		pre_image -> Text,
	}
}

table! {
	initiated_events (id) {
		id -> Int4,
		bridge_transfer_id -> Text,
		initiator -> Text,
		recipient -> Text,
		hash_lock -> Text,
		time_lock -> BigInt,
		amount -> Numeric,
		state -> Int2,
	}
}

table! {
	locked_events (id) {
		id -> Int4,
		bridge_transfer_id -> Text,
		initiator -> Text,
		recipient -> Text,
		hash_lock -> Text,
		time_lock -> BigInt,
		amount -> Numeric,
	}
}

table! {
	initiator_completed_events (id) {
		id -> Int4,
		bridge_transfer_id -> Text,
	}
}

table! {
	counter_party_completed_events (id) {
		id -> Int4,
		bridge_transfer_id -> Text,
		pre_image -> Text,
	}
}

table! {
	cancelled_events (id) {
		id -> Int4,
		bridge_transfer_id -> Text,
	}
}

table! {
	refunded_events (id) {
		id -> Int4,
		bridge_transfer_id -> Text,
	}
}
