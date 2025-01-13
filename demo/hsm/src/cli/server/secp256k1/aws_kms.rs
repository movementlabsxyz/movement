use crate::server::create_server;
use axum::Server;
use clap::Parser;
use movement_signer::cryptography::secp256k1::Secp256k1;
use movement_signer::key::Key;
use movement_signer::key::SignerBuilder;
use movement_signer::Signer;
use movement_signer_aws_kms::hsm::key::Builder;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Parser, Clone)]
#[clap(rename_all = "kebab-case", about = "Runs signing app for secp256k1 against AWS KMS")]
pub struct AwsKms {
	canonical_key: String,
	#[arg(long)]
	create_key: bool,
}

impl AwsKms {
	pub async fn run(&self) -> Result<(), anyhow::Error> {
		// build the hsm
		let key = Key::try_from_canonical_string(self.canonical_key.as_str())
			.map_err(|e| anyhow::anyhow!(e))?;
		let builder = Builder::<Secp256k1>::new().create_key(self.create_key);
		let hsm = Signer::new(builder.build(key).await?);

		// Build the server
		let server_hsm = Arc::new(Mutex::new(hsm));

		let app = create_server(server_hsm);
		let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
		println!("Server listening on {}", addr);

		Server::bind(&addr).serve(app.into_make_service()).await?;

		Ok(())
	}
}
