use crate::schema::bridge_transfers;
use bigdecimal::BigDecimal;
use diesel::{Insertable, Queryable};

#[derive(Queryable, Insertable)]
#[table_name = "bridge_transfers"]
pub struct BridgeTransfer {
	pub id: i32,
	pub source_chain: String,
	pub source_address: String,
	pub destination_chain: String,
	pub destination_address: String,
	pub bridge_transfer_id: String,
	pub hash_lock: String,
	pub amount: BigDecimal,
}
