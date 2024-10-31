use crate::models::BridgeTransfer;
use crate::schema::bridge_transfers;
use diesel::pg::PgConnection;
use diesel::prelude::*;

pub fn create_bridge_transfer(
	conn: &mut PgConnection,
	new_transfer: BridgeTransfer,
) -> QueryResult<usize> {
	diesel::insert_into(bridge_transfers::table).values(&new_transfer).execute(conn)
}
