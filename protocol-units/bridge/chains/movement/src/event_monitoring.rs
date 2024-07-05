/// Trait for event types which returns trasaction_hash and block_height
pub trait TxSpecificData {
	/// return block_height
	fn block_height(&self) -> String;
	/// return transaction_hash
	fn transaction_hash(&self) -> String;
}
