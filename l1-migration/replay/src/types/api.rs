use aptos_rest_client::Client;
use std::ops::Deref;

pub struct AptosRestClient(Client);

impl AptosRestClient {
	pub fn new(url: &str) -> Result<Self, anyhow::Error> {
		let client = Client::new(
			url.parse()
				.map_err(|e| anyhow::anyhow!("Failed to parse Aptos rest api url: {}", e))?,
		);
		Ok(Self(client))
	}
}

impl Deref for AptosRestClient {
	type Target = Client;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}
