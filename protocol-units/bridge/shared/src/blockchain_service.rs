use crate::{
	bridge_contracts::{BridgeContractCounterparty, BridgeContractInitiator},
	bridge_monitoring::{BridgeContractCounterpartyMonitoring, BridgeContractInitiatorMonitoring},
};

pub trait BlockchainService {
	type Address;
	type Hash;

	type InitiatorContract: BridgeContractInitiator<Self::Address, Self::Hash>;
	type InitiatorMonitoring: BridgeContractInitiatorMonitoring<Self::Address, Self::Hash>;

	type CounterpartyContract: BridgeContractCounterparty<Self::Address, Self::Hash>;
	type CounterpartyMonitoring: BridgeContractCounterpartyMonitoring<Self::Address, Self::Hash>;

	fn initiator_contract(&self) -> &Self::InitiatorContract;
	fn initiator_monitoring(&self) -> &Self::InitiatorMonitoring;
	fn counterparty_contract(&self) -> &Self::CounterpartyContract;
	fn counterparty_monitoring(&self) -> &Self::CounterpartyMonitoring;
}
