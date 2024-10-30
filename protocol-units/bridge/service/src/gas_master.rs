use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum GasMasterError {
	#[error("Failed to get gas price")]
	GetGasPriceError,
	#[error("Failed to get priority gas fee estimate")]
	GetPriorityGasFeeEstimateError,
}

pub type GasMasterResult<T> = Result<T, GasMasterError>;

/// `GasMaster` is a trait that provides an interface for retrieving gas prices and calculating dynamic fees
/// on anuy implementing clien.
#[async_trait::async_trait]
pub trait GasMaster {
	/// Retrieve the current base gas price from the implementing network.
	async fn get_base_gas_price(&self) -> GasMasterResult<u64>;

	/// Retrieve priority fee estimates based on recent blocks.
	async fn get_priority_gas_fee_estimate(&self) -> GasMasterResult<u64>;

	/// Calculate the dynamic fee, adjusting by a specified percentage.
	/// `percentage_adjustment` is the percentage to increase or decrease the base gas price.
	async fn calculate_dynamic_fee(&self) -> GasMasterResult<u64>;
}
