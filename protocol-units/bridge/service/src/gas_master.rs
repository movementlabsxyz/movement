use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum GasMasterError {
	#[error("Failed to get gas price")]
	GetGasPriceError,
}

pub type GasMasterResult<T> = Result<T, GasMasterError>;

#[async_trait::async_trait]
pub trait GasMaster {
	/// Retrieve the current base gas price from the implementing network.
	async fn get_gas_price(&self) -> GasMasterResult<u64>;

	/// Retrieve priority fee estimates based on recent blocks.
	async fn get_priority_gas_fee_estiate(&self) -> GasMasterResult<u64>;

	async fn calculate_dynamic_fee(&self) -> GasMasterResult<u64>;
}
