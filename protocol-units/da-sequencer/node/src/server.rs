/// Runs the server
async fn run_server(&self) -> Result<(), anyhow::Error> {
	let reflection = tonic_reflection::server::Builder::configure()
		.register_encoded_file_descriptor_set(movement_da_light_node_proto::FILE_DESCRIPTOR_SET)
		.build_v1()?;

	let address = self.try_service_address()?;
	info!("Server listening on: {}", address);
	Server::builder()
		.max_frame_size(1024 * 1024 * 16 - 1)
		.accept_http1(true)
		.add_service(LightNodeServiceServer::new(self.clone()))
		.add_service(reflection)
		.serve(address.parse()?)
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

	info!("Running server and background tasks.");
	tokio::try_join!(server, background_tasks)?;

	Ok(())
}
