use crate::{cryptography::Secp256k1, hsm, server::create_server};
use axum::Server;
use clap::Parser;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Parser, Clone)]
#[clap(rename_all = "kebab-case", about = "Runs signing app for secp256k1 against AWS KMS")]
pub struct AwsKms {}

impl AwsKms {
	pub async fn run(&self) -> Result<(), anyhow::Error> {
		let hsm = hsm::aws_kms::AwsKms::<Secp256k1>::try_from_env()
			.await?
			.create_key()
			.await?
			.fill_with_public_key()
			.await?;
		let server_hsm = Arc::new(Mutex::new(hsm));

		let app = create_server(server_hsm);
		let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
		println!("Server listening on {}", addr);

		Server::bind(&addr).serve(app.into_make_service()).await?;

		Ok(())
	}
}
