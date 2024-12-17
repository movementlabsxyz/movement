use hsm_demo::{action_stream, hsm, Application};

#[tokio::main]
pub async fn main() -> Result<(), anyhow::Error> {
	let random_stream = action_stream::random::Random;
	let notify_verify_stream = action_stream::notify_verify::NotifyVerify::new();
	let join_stream = action_stream::join::Join::new(vec![
		Box::new(random_stream),
		Box::new(notify_verify_stream),
	]);

	let hsm = hsm::aws_kms::AwsKms::try_from_env()
		.await?
		.create_key()
		.await?
		.fill_with_public_key()
		.await?;

	let mut app = Application::new(Box::new(hsm), Box::new(join_stream));

	app.run().await?;

	Ok(())
}
