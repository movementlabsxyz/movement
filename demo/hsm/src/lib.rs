pub mod action_stream;
pub mod hsm;

/// A collection of bytes.
#[derive(Debug)]
pub struct Bytes(pub Vec<u8>);

/// A signature.
#[derive(Debug)]
pub struct Signature(pub Bytes);

/// A message to be signed or verified.
pub enum Message {
	Sign(Bytes),
	Verify(Bytes, Bytes),
}

/// A stream of messages to be signed or verified.
#[async_trait::async_trait]
pub trait ActionStream {
	async fn next(&mut self) -> Option<Message>;
}

/// An HSM capable of signing and verifying messages.
#[async_trait::async_trait]
pub trait Hsm {
	async fn sign(&self, message: Bytes) -> Result<Signature, anyhow::Error>;
	async fn verify(&self, message: Bytes, signature: Signature) -> Result<bool, anyhow::Error>;
}

/// An application which reads a stream of messages to either sign or verify.
pub struct Application {
	hsm: Box<dyn Hsm>,
	stream: Box<dyn ActionStream>,
}

/// The application implementation.
impl Application {
	/// Creates a new application.
	pub fn new(hsm: Box<dyn Hsm>, stream: Box<dyn ActionStream>) -> Self {
		Self { hsm, stream }
	}

	/// Runs the application.
	pub async fn run(&mut self) {
		while let Some(message) = self.stream.next().await {
			match message {
				Message::Sign(message) => {
					let signature = self.hsm.sign(message).await;
					println!("Signed message: {:?}", signature);
				}
				Message::Verify(message, signature) => {
					let verified = self.hsm.verify(message, Signature(signature)).await;
					println!("Verified message: {:?}", verified);
				}
			}
		}
	}
}
