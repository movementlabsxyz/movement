use super::{LightNode, LightNodeRuntime};
use godfig::{backend::config_file::ConfigFile, Godfig};
use movement_da_light_node_celestia::da::Da as CelestiaDa;
use movement_da_light_node_digest_store::da::Da as DigestStoreDa;
use movement_da_light_node_verifier::signed::InKnownSignersVerifier;
use movement_da_util::config::Config;
use movement_signer::cryptography::secp256k1::Secp256k1;
use movement_signer_loader::identifiers::LoadedSigner;

pub struct Manager<LightNode>
where
	LightNode: LightNodeRuntime,
{
	godfig: Godfig<Config, ConfigFile>,
	_marker: std::marker::PhantomData<LightNode>,
}

// Implements a very simple manager using a marker strategy pattern.

impl
	Manager<
		LightNodeV1<
			LoadedSigner<Secp256k1>,
			Secp256k1,
			DigestStoreDa<CelestiaDa>,
			InKnownSignersVerifier<Secp256k1>,
		>,
	>
{
	pub async fn new(file: tokio::fs::File) -> Result<Self, anyhow::Error> {
		let godfig = Godfig::new(
			ConfigFile::new(file),
			vec![
				"celestia_da_light_node_config".to_string(), // in this example this comes from the structuring of the config file
			],
		);
		Ok(Self { godfig, _marker: std::marker::PhantomData })
	}

	pub async fn try_light_node(
		&self,
	) -> Result<
		LightNodeV1<
			LoadedSigner<Secp256k1>,
			Secp256k1,
			DigestStoreDa<CelestiaDa>,
			InKnownSignersVerifier<Secp256k1>,
		>,
		anyhow::Error,
	> {
		let config = self.godfig.try_wait_for_ready().await?;
		LightNode::try_from_config(config).await
	}

	pub async fn try_run(&self) -> Result<(), anyhow::Error> {
		let light_node = self.try_light_node().await?;
		light_node.run().await
	}
}
