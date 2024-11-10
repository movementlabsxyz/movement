use diesel::prelude::*;

table! {
	lock_bridge_transfers (id) {
		id -> Int4,
		bridge_transfer_id -> Text,
		hash_lock -> Text,
		initiator -> Text,
		recipient -> Text,
		amount -> Numeric,
		created_at -> Timestamp,
	}
}

table! {
	wait_and_complete_initiators (id) {
		id -> Int4,
		wait_time_secs -> BigInt,
		pre_image -> Text,
		created_at -> Timestamp,
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
		created_at -> Timestamp,
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
		created_at -> Timestamp,
	}
}

table! {
	initiator_completed_events (id) {
		id -> Int4,
		bridge_transfer_id -> Text,
		created_at -> Timestamp,
	}
}

table! {
	counter_party_completed_events (id) {
		id -> Int4,
		bridge_transfer_id -> Text,
		pre_image -> Text,
		created_at -> Timestamp,
	}
}

table! {
	cancelled_events (id) {
		id -> Int4,
		bridge_transfer_id -> Text,
		created_at -> Timestamp,
	}
}

table! {
	refunded_events (id) {
		id -> Int4,
		bridge_transfer_id -> Text,
		created_at -> Timestamp,
	}
}
