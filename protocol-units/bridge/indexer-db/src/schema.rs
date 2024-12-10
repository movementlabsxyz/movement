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
