use diesel::prelude::*;

table! {
	bridge_transfers (id) {
		id -> Int4,
		source_chain -> Varchar,
		source_address -> Varchar,
		destination_chain -> Varchar,
		destination_address -> Varchar,
		bridge_transfer_id -> Varchar,
		hash_lock -> Varchar,
		amount -> Numeric,
	}
}
