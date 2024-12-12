use diesel::prelude::*;

table! {
	initiated_events (id) {
		id -> Int4,
		bridge_transfer_id -> Text,
		initiator -> Text,
		recipient -> Text,
		amount -> Numeric,
		nonce -> Numeric,
		created_at -> Timestamp,
	}
}

table! {
	completed_events (id) {
		id -> Int4,
		bridge_transfer_id -> Text,
		initiator -> Text,
		recipient -> Text,
		amount -> Numeric,
		nonce -> Numeric,
		created_at -> Timestamp,
	}
}

table! {
	complete_bridge_transfers (id) {
		id -> Int4,
		bridge_transfer_id -> Text,
		initiator -> Text,
		recipient -> Text,
		amount -> Numeric,
		nonce -> Numeric,
		created_at -> Timestamp,
	}
}

table! {
	completed_remove_state (id) {
		id -> Int4,
		bridge_transfer_id -> Text,
		created_at -> Timestamp,
	}
}

table! {
	abort_replay_transfers (id) {
		id -> Int4,
		bridge_transfer_id -> Text,
		initiator -> Text,
		recipient -> Text,
		amount -> Numeric,
		nonce -> Numeric,
		wait_time_sec -> Numeric,
		created_at -> Timestamp,
	}
}
