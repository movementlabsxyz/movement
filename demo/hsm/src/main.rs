use hsm_demo::{action_stream, hsm, Application};

#[tokio::main]
pub async fn main() -> Result<(), anyhow::Error> {
	let stream = action_stream::random::Random;

	let hsm = hsm::hashi_corp_vault::HashiCorpVault::new()

	Ok(())
}
