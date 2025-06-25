use movement_celestia_da_light_node::{LightNode, Manager};
use movement_da_light_node_celestia::da::Da as CelestiaDa;
use movement_da_light_node_digest_store::da::Da as DigestStoreDa;
use movement_da_light_node_verifier::signed::InKnownSignersVerifier;
use movement_signer::cryptography::secp256k1::Secp256k1;
use movement_signer_loader::LoadedSigner;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let config_path = dot_movement.get_config_json_path();
	let config_file = tokio::fs::File::open(config_path).await?;
	// todo: consider whether LightNode implementation should encapsulate signing type

	let manager = Manager::<
		LightNode<
			LoadedSigner<Secp256k1>,
			Secp256k1,
			DigestStoreDa<Secp256k1, CelestiaDa<Secp256k1>>,
			InKnownSignersVerifier<Secp256k1>,
		>,
	>::new(config_file)
	.await?;
	manager.try_run().await?;

	Ok(())
}
