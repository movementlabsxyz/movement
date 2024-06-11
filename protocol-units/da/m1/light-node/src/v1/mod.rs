#[cfg(all(feature = "sequencer", feature = "dynamic"))]
pub mod dynamic;
pub mod passthrough;
#[cfg(feature = "sequencer")]
pub mod sequencer;

#[cfg(not(feature = "sequencer"))]
pub use passthrough::*;

#[cfg(all(feature = "sequencer", not(feature = "dynamic")))]
pub use sequencer::*;

#[cfg(all(feature = "dynamic", feature = "sequencer"))]
pub use dynamic::*;

use m1_da_light_node_grpc::light_node_service_server::{LightNodeService, LightNodeServiceServer};
use tonic::transport::Server;

pub trait LightNodeV1Operations: LightNodeService + Send + Sync + Sized + Clone {
	/// Initializes from environment variables.
	async fn try_from_env_toml_file() -> Result<Self, anyhow::Error>;

	/// Runs the background tasks.
	async fn run_background_tasks(&self) -> Result<(), anyhow::Error>;

	/// Runs the server
	async fn run_server(&self) -> Result<(), anyhow::Error> {
		let reflection = tonic_reflection::server::Builder::configure()
			.register_encoded_file_descriptor_set(m1_da_light_node_grpc::FILE_DESCRIPTOR_SET)
			.build()?;

		let env_addr =
			std::env::var("M1_DA_LIGHT_NODE_ADDR").unwrap_or_else(|_| "0.0.0.0:30730".to_string());
		let addr = env_addr.parse()?;

		Server::builder()
			.accept_http1(true)
			.add_service(LightNodeServiceServer::new(self.clone()))
			.add_service(reflection)
			.serve(addr)
			.await?;

		Ok(())
	}

	/// Runs the server and the background tasks.
	async fn run(self) -> Result<(), anyhow::Error> {
		let background_handle = self.run_background_tasks();

		let background_tasks = async move {
			background_handle.await?;
			Ok::<_, anyhow::Error>(())
		};
		let server = self.run_server();

		tokio::try_join!(server, background_tasks)?;

		Ok(())
	}
}
