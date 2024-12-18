pub mod action_stream;
pub mod cli;
pub mod cryptography;
pub mod hsm;
pub mod server;

/// A collection of bytes.
#[derive(Debug, Clone)]
pub struct Bytes(pub Vec<u8>);

/// A signature.
#[derive(Debug, Clone)]
pub struct Signature(pub Bytes);

/// A public key.
#[derive(Debug, Clone)]
pub struct PublicKey(pub Bytes);

#[derive(Debug, Clone)]
/// A message to be signed or verified.
pub enum Message {
	Sign(Bytes),
	Verify(Bytes, PublicKey, Signature),
}

/// A stream of messages to be signed or verified.
#[async_trait::async_trait]
pub trait ActionStream {
	/// Notifies the stream of a message emitted from elsewhere in the system.
	async fn notify(&mut self, message: Message) -> Result<(), anyhow::Error>;

	/// Gets the message to act upon.
	async fn next(&mut self) -> Result<Option<Message>, anyhow::Error>;
}

/// An HSM capable of signing and verifying messages.
#[async_trait::async_trait]
pub trait Hsm {
	async fn sign(&self, message: Bytes) -> Result<(Bytes, PublicKey, Signature), anyhow::Error>;
	async fn verify(
		&self,
		message: Bytes,
		public_key: PublicKey,
		signature: Signature,
	) -> Result<bool, anyhow::Error>;
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
	pub async fn run(&mut self) -> Result<(), anyhow::Error> {
		while let Some(message) = self.stream.next().await? {
			println!("RECEIVED: {:?}", message);
			match message {
				Message::Sign(message) => {
					println!("SIGNING: {:?}", message);
					let (message, public_key, signature) = self.hsm.sign(message).await?;
					println!("SIGNED:\n{:?}\n{:?}\n{:?}", message, public_key, signature);
					self.stream.notify(Message::Verify(message, public_key, signature)).await?;
				}
				Message::Verify(message, public_key, signature) => {
					println!("VERIFYING:\n{:?}\n{:?}\n{:?}", message, public_key, signature);
					let verified = self.hsm.verify(message, public_key, signature).await?;
					println!("VERIFIED: {:?}", verified);
				}
			}
		}
		Ok(())
	}
}
