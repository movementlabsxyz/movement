// Run the e2e signing Test.
// e2e signing test are run using real node: 'ex Anvil and Suzuka node).

// Use the local signer to sign Eth Tx. Can be run in the CI.
#[tokio::test]
async fn e2e_eth_signing_local() -> Result<(), anyhow::Error> {
	todo!()
}

// Use the AWS KMS signer to sign Eth Tx. AWS auth env var must be set to run the test.
#[tokio::test]
async fn e2e_eth_signing_awskms() -> Result<(), anyhow::Error> {
	todo!()
}
