use bridge_service::types::Amount;
use bridge_util::chains::bridge_contracts::BridgeContractError;
use bridge_util::chains::bridge_contracts::BridgeContractResult;
use bridge_util::chains::bridge_contracts::BridgeTransferCompletedDetails;
use bridge_util::chains::bridge_contracts::BridgeTransferInitiatedDetails;
use bridge_util::types::AddressError;
use bridge_util::types::BridgeAddress;
use bridge_util::types::Nonce;
use bridge_util::BridgeContractEvent;
use bridge_util::BridgeContractMonitoring;
use bridge_util::BridgeRelayerContract;
use bridge_util::BridgeTransferId;
use futures::SinkExt;
use futures::{
	channel::mpsc::{UnboundedReceiver, UnboundedSender},
	Stream, StreamExt,
};
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::{pin::Pin, task::Poll};
use tiny_keccak::{Hasher, Keccak};
use tokio::sync::mpsc;
use tokio::sync::oneshot;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct MockAddress(pub Vec<u8>);

impl From<MockAddress> for Vec<u8> {
	fn from(address: MockAddress) -> Self {
		address.0
	}
}

impl From<BridgeAddress<MockAddress>> for MockAddress {
	fn from(address: BridgeAddress<MockAddress>) -> Self {
		address.0
	}
}

impl std::ops::Deref for MockAddress {
	type Target = Vec<u8>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl TryFrom<Vec<u8>> for MockAddress {
	type Error = AddressError;

	fn try_from(vec: Vec<u8>) -> Result<Self, Self::Error> {
		Ok(MockAddress(vec))
	}
}

fn calculate_bridge_transfer_id(
	initiator: &[u8],
	recipient: &[u8],
	amount: Amount,
	nonce: Nonce,
) -> BridgeTransferId {
	let mut hasher = Keccak::v256();
	hasher.update(initiator);
	hasher.update(recipient);
	hasher.update(&amount.to_le_bytes());
	hasher.update(&nonce.0.to_le_bytes());
	let mut output = [0u8; 32];
	hasher.finalize(&mut output);
	BridgeTransferId(output)
}

#[derive(Clone)]
pub struct RelayerMockClient {
	sender: UnboundedSender<BridgeContractResult<BridgeContractEvent<MockAddress>>>,
	complete_notifier:
		tokio::sync::mpsc::Sender<BridgeContractResult<BridgeContractEvent<MockAddress>>>,
	send_retry: Arc<AtomicUsize>,
}

impl RelayerMockClient {
	pub fn build(
		send_retry: usize,
		sender: UnboundedSender<BridgeContractResult<BridgeContractEvent<MockAddress>>>,
	) -> (Self, tokio::sync::mpsc::Receiver<BridgeContractResult<BridgeContractEvent<MockAddress>>>)
	{
		let (notifier_sender, notifier_listener) = tokio::sync::mpsc::channel::<
			BridgeContractResult<BridgeContractEvent<MockAddress>>,
		>(100);
		(
			RelayerMockClient {
				sender,
				complete_notifier: notifier_sender,
				send_retry: Arc::new(AtomicUsize::new(send_retry)),
			},
			notifier_listener,
		)
	}
}

#[async_trait::async_trait]
impl BridgeRelayerContract<MockAddress> for RelayerMockClient {
	async fn complete_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId,
		initiator: BridgeAddress<Vec<u8>>,
		recipient: BridgeAddress<MockAddress>,
		amount: Amount,
		nonce: Nonce,
	) -> BridgeContractResult<()> {
		//manage Tx send error simulation
		let retry = self.send_retry.fetch_sub(1, Ordering::Acquire);
		// if retry unwrap (> 5000) should be considered as 0.
		if retry != 0 && retry < 5000 {
			self.complete_notifier
				.send(Err(BridgeContractError::OnChainError("Retry.".to_string())))
				.await
				.unwrap();
			return Err(BridgeContractError::OnChainError("Send Tx failed, retry".to_string()));
		}

		//verify the transfer Id
		let calcuated_bridge_transfer_id =
			calculate_bridge_transfer_id(&initiator.0, &recipient.0 .0, amount, nonce);
		if bridge_transfer_id != calcuated_bridge_transfer_id {
			self.complete_notifier
				.send(Err(BridgeContractError::OnChainError("Bad transfer Id.".to_string())))
				.await
				.unwrap();
			return Err(BridgeContractError::OnChainError(
				"Transfer Id verification failed".to_string(),
			));
		}

		let details: BridgeTransferCompletedDetails<MockAddress> = BridgeTransferCompletedDetails {
			bridge_transfer_id,
			initiator: BridgeAddress(initiator.0),
			recipient: BridgeAddress(recipient.0),
			nonce,
			amount,
		};
		let event = BridgeContractEvent::Completed(details);
		self.sender.send(Ok(event.clone())).await.unwrap();
		self.complete_notifier.send(Ok(event)).await.unwrap();

		Ok(())
	}
	async fn get_bridge_transfer_details_with_nonce(
		&mut self,
		nonce: Nonce,
	) -> BridgeContractResult<Option<BridgeTransferInitiatedDetails<MockAddress>>> {
		todo!()
	}
	async fn is_bridge_transfer_completed(
		&mut self,
		bridge_transfer_id: BridgeTransferId,
	) -> BridgeContractResult<bool> {
		todo!()
	}
}

pub struct MockMonitoring {
	listener: UnboundedReceiver<BridgeContractResult<BridgeContractEvent<MockAddress>>>,
}

impl MockMonitoring {
	pub fn build(
		listener: UnboundedReceiver<BridgeContractResult<BridgeContractEvent<MockAddress>>>,
		mut health_check_rx: mpsc::Receiver<oneshot::Sender<bool>>,
	) -> Self {
		//Start Health check loop
		tokio::spawn({
			async move {
				loop {
					//Check if there's a health check request
					if let Some(tx) = health_check_rx.recv().await {
						if let Err(err) = tx.send(true) {
							tracing::warn!("Mock Heath check send on oneshot channel failed:{err}");
						}
					}
				}
			}
		});

		MockMonitoring { listener }
	}
}

impl BridgeContractMonitoring for MockMonitoring {
	type Address = MockAddress;
}

impl Stream for MockMonitoring {
	type Item = BridgeContractResult<BridgeContractEvent<MockAddress>>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut std::task::Context) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();
		this.listener.poll_next_unpin(cx)
	}
}

async fn initiate_bridge_transfer(
	initiator: MockAddress,
	recipient: MockAddress,
	amount: Amount,
	nonce: Nonce,
	sender: &mut UnboundedSender<BridgeContractResult<BridgeContractEvent<MockAddress>>>,
) -> BridgeTransferId {
	let bridge_transfer_id =
		calculate_bridge_transfer_id(&initiator.0, &recipient.0, amount, nonce);
	let details = BridgeTransferInitiatedDetails {
		bridge_transfer_id,
		initiator: BridgeAddress(initiator.clone()),
		recipient: BridgeAddress(recipient.clone().0),
		nonce,
		amount,
	};
	let event = BridgeContractEvent::Initiated(details);

	sender.send(Ok(event)).await.unwrap();
	bridge_transfer_id
}

#[tokio::test]
async fn test_relayer_logic() -> Result<(), anyhow::Error> {
	use tracing_subscriber::EnvFilter;
	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	let l1_initiator_address = MockAddress(vec![11]);
	let l2_recipient_address = MockAddress(vec![22]);

	let (mut l1_sender, l1_listener) = futures::channel::mpsc::unbounded::<
		BridgeContractResult<BridgeContractEvent<MockAddress>>,
	>();
	let (l1_health_tx, l1_health_rx) = tokio::sync::mpsc::channel(10);
	let l1_monitor = MockMonitoring::build(l1_listener, l1_health_rx);
	let (l2_sender, l2_listener) = futures::channel::mpsc::unbounded::<
		BridgeContractResult<BridgeContractEvent<MockAddress>>,
	>();
	let (l2_relayer_client, mut l2_mock_notifier) = RelayerMockClient::build(2, l2_sender.clone());

	let (_l2_health_tx, l2_health_rx) = tokio::sync::mpsc::channel(10);
	let l2_monitor = MockMonitoring::build(l2_listener, l2_health_rx);

	// Start relay in L1-> L2 direction
	let _ = tokio::spawn({
		async move {
			bridge_service::relayer::run_relayer_one_direction(
				"L1->L2",
				l1_monitor,
				l2_relayer_client,
				l2_monitor,
			)
			.await
		}
	});
	println!("ICI3");

	// Test send Tx fail 2 time L1-> L2
	let l1_transfer_id = initiate_bridge_transfer(
		l1_initiator_address.clone(),
		l2_recipient_address.clone(),
		Amount(11),
		Nonce(12),
		&mut l1_sender,
	)
	.await;

	// Get retry event.
	let event = tokio::time::timeout(std::time::Duration::from_secs(5), l2_mock_notifier.recv())
		.await
		.expect("L2 commplete not call by the relayer.");
	assert!(event.is_some());
	let event = event.unwrap();
	assert!(event.is_err());

	let event = tokio::time::timeout(std::time::Duration::from_secs(15), l2_mock_notifier.recv())
		.await
		.expect("L2 commplete not call by the relayer.");
	assert!(event.is_some());
	let event = event.unwrap();
	assert!(event.is_err());
	//get ok transfer completed
	let event = tokio::time::timeout(std::time::Duration::from_secs(15), l2_mock_notifier.recv())
		.await
		.expect("L2 commplete not call by the relayer.");
	assert!(event.is_some());
	let event = event.unwrap();
	assert!(event.is_ok());
	let event = event.unwrap();
	assert_eq!(event.bridge_transfer_id(), l1_transfer_id);

	// Test Happy path L1->L2
	// Simulate Initiate transfer event.
	let l1_transfer_id = initiate_bridge_transfer(
		l1_initiator_address,
		l2_recipient_address,
		Amount(111),
		Nonce(112),
		&mut l1_sender,
	)
	.await;
	// Wait for completed notification.
	let event = tokio::time::timeout(std::time::Duration::from_secs(5), l2_mock_notifier.recv())
		.await
		.expect("L2 commplete not call by the relayer.");
	assert!(event.is_some());
	let event = event.unwrap();
	assert!(event.is_ok());
	let event = event.unwrap();
	assert_eq!(event.bridge_transfer_id(), l1_transfer_id);

	Ok(())
}
