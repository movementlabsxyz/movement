use crate::eth_client::Client;
use crate::McrSettlementClientOperations;
use anyhow::Context;
use godfig::{backend::config_file::ConfigFile, Godfig};
use mcr_settlement_config::Config;
use movement_types::block::BlockCommitment;
use movement_types::block::Commitment;
use movement_types::block::Id;
use tokio_stream::StreamExt;

#[tokio::test]
pub async fn test_client_settlement() -> Result<(), anyhow::Error> {
	use tracing_subscriber::EnvFilter;

	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let config_file = dot_movement.try_get_or_create_config_file().await?;

	// get a matching godfig object
	let godfig: Godfig<Config, ConfigFile> =
		Godfig::new(ConfigFile::new(config_file), vec!["mcr_settlement".to_string()]);
	let config: Config = godfig.try_wait_for_ready().await?;
	let rpc_url = config.eth_rpc_connection_url();

	let testing_config = config.testing.as_ref().context("Testing config not defined.")?;
	let deploy_config = config.deploy.as_ref().context("Deploy config not defined.")?;

	// Genesis ceremony has been run during the setup.

	// Build client 1 and send the first commitment.
	//let settlement_config =
	let config1 = Config {
		settle: mcr_settlement_config::common::settlement::Config {
			signer_private_key: config.settle.signer_private_key.to_string(),
			..config.settle.clone()
		},
		..config.clone()
	};
	let client1 = Client::build_with_config(&config1).await.unwrap();

	let mut client1_stream = client1.stream_block_commitments().await.unwrap();
	// Client post a new commitment. Commitment height 1 already posted by the setup.
	let commitment = BlockCommitment::new(2, Id::new([2; 32]), Commitment::new([3; 32]));

	let res = client1.post_block_commitment(commitment.clone()).await;
	assert!(res.is_ok(), "post_block_commitment1 client1 result:{res:?}");

	// No notification, quorum is not reached
	let res =
		tokio::time::timeout(tokio::time::Duration::from_secs(5), client1_stream.next()).await;
	assert!(res.is_err());

	// Build client 2 and send the second commitment.
	let config2 = Config {
		settle: mcr_settlement_config::common::settlement::Config {
			signer_private_key: testing_config
				.well_known_account_private_keys
				.get(2)
				.context("No well known account")?
				.to_string(),
			..config.settle.clone()
		},
		..config.clone()
	};
	let client2 = Client::build_with_config(&config2).await.unwrap();

	let mut client2_stream = client2.stream_block_commitments().await.unwrap();

	// Client post a new commitment
	let res = client2.post_block_commitment(commitment).await;
	assert!(res.is_ok(), "post_block_commitment1 client2 result:{res:?}");

	// Now we move to block 2 and make some commitment just to trigger the epochRollover
	let commitment2 = BlockCommitment::new(3, Id::new([4; 32]), Commitment::new([5; 32]));

	let res = client2.post_block_commitment(commitment2.clone()).await;
	assert!(res.is_ok(), "post_block_commitment2 client2 result:{res:?}");

	// Validate that the accepted commitment stream gets the event.
	let event = tokio::time::timeout(tokio::time::Duration::from_secs(7), client1_stream.next())
		.await
		.unwrap()
		.unwrap()
		.unwrap();
	assert_eq!(event.commitment().as_bytes()[0], 3);
	assert_eq!(event.block_id().as_bytes()[0], 2);

	let event = tokio::time::timeout(tokio::time::Duration::from_secs(7), client2_stream.next())
		.await
		.unwrap()
		.unwrap()
		.unwrap();
	assert_eq!(event.commitment().as_bytes()[0], 3);
	assert_eq!(event.block_id().as_bytes()[0], 2);

	// Test post batch commitment
	// Post the complementary batch on height 2 and one on height 3
	let commitment3 = BlockCommitment::new(4, Id::new([6; 32]), Commitment::new([7; 32]));
	let res = client1.post_block_commitment_batch(vec![commitment2, commitment3]).await;
	assert!(res.is_ok(), "post_block_commitment2 client1 result:{res:?}");
	// Validate that the commitments stream gets the event.
	let event = tokio::time::timeout(tokio::time::Duration::from_secs(5), client1_stream.next())
		.await
		.unwrap()
		.unwrap()
		.unwrap();
	assert_eq!(event.commitment().as_bytes()[0], 5);
	assert_eq!(event.block_id().as_bytes()[0], 4);
	let event = tokio::time::timeout(tokio::time::Duration::from_secs(7), client2_stream.next())
		.await
		.unwrap()
		.unwrap()
		.unwrap();
	assert_eq!(event.commitment().as_bytes()[0], 5);
	assert_eq!(event.block_id().as_bytes()[0], 4);

	// Test get_commitment_at_height
	let commitment = client1.get_commitment_at_height(2).await?;
	assert!(commitment.is_some());
	let commitment = commitment.unwrap();
	assert_eq!(commitment.commitment().as_bytes()[0], 3);
	assert_eq!(commitment.block_id().as_bytes()[0], 2);
	let commitment = client1.get_commitment_at_height(10).await?;
	assert_eq!(commitment, None);

	Ok(())
}
