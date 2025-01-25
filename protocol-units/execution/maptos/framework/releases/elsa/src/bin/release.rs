use aptos_framework_elsa_release::Elsa;
use maptos_framework_release_util::Release;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	let elsa = Elsa::new();
	let release_bundle = elsa.release()?;

	Ok(())
}
