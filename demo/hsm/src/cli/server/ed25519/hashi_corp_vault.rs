use crate::server::create_server;
use axum::Server;
use clap::Parser;
use movement_signer::cryptography::ed25519::Ed25519;
use movement_signer::key::Key;
use movement_signer::key::SignerBuilder;
use movement_signer::Signer;
use movement_signer_hashicorp_vault::hsm::key::Builder;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Parser, Clone)]
#[clap(rename_all = "kebab-case", about = "Runs signing app for ed25519 against HashiCorp Vault")]
pub struct HashiCorpVault {
	canonical_key: String,
	#[arg(long)]
	create_key: bool,
}

impl HashiCorpVault {
	pub async fn run(&self) -> Result<(), anyhow::Error> {
		// build the hsm
		let key = Key::try_from_canonical_string(self.canonical_key.as_str())
			.map_err(|e| anyhow::anyhow!(e))?;
		let builder = Builder::<Ed25519>::new().create_key(self.create_key);
		let hsm = Signer::new(builder.build(key).await?);

		// build the server
		let server_hsm = Arc::new(Mutex::new(hsm));
		let app = create_server(server_hsm);
		let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
		println!("Server listening on {}", addr);

		Server::bind(&addr).serve(app.into_make_service()).await?;

		Ok(())
	}
}
