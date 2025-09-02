use anyhow::Context;
use aptos_rest_client::Client;
use std::ops::Deref;
use std::sync::Arc;

#[derive(Clone)]
pub struct AptosRestClient(Arc<Client>);

impl AptosRestClient {
	pub async fn try_connect(url: &str) -> Result<Self, anyhow::Error> {
		let client = try_connect("Aptos", url).await?;
		Ok(Self(Arc::new(client)))
	}
}

impl Deref for AptosRestClient {
	type Target = Client;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

#[derive(Clone)]
pub struct MovementRestClient(Arc<Client>);

impl MovementRestClient {
	pub async fn try_connect(url: &str) -> Result<Self, anyhow::Error> {
		let client = try_connect("Movement", url).await?;
		Ok(Self(Arc::new(client)))
	}
}

impl Deref for MovementRestClient {
	type Target = Client;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

async fn try_connect(name: &str, url: &str) -> Result<Client, anyhow::Error> {
	let client = Client::new(
		url.parse()
			.map_err(|e| anyhow::anyhow!("Failed to parse {} rest api url: {}", name, e))?,
	);
	client
		.get_index_bcs()
		.await
		.context(format!("{} rest api unreachable.", name))?;
	Ok(client)
}
