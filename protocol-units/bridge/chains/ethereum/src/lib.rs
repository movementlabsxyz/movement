pub struct EthHash(); // Alloy type inside
					  //
struct EthInitiatorContractMonitoring<A, H> {
	listener: UnboundedReceiver<AbstractBlockchainEvent<A, H>>,
}
