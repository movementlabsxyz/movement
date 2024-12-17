use anyhow::Result;

pub mod aws;
pub mod hashivault;

/// A collection of bytes.
#[derive(Debug, Clone)]
pub struct Bytes(pub Vec<u8>);

/// A signature.
#[derive(Debug)]
pub struct Signature(pub Bytes);

/// A message to be signed or verified.
pub enum Message {
	Sign(Bytes),
	Verify(Bytes, Signature),
}

/// A stream of messages to be signed or verified.
#[async_trait::async_trait]
pub trait ActionStream {
	async fn next(&mut self) -> Option<Message>;
}

/// An HSM capable of signing and verifying messages.
#[async_trait::async_trait]
pub trait Hsm {
	async fn sign(&self, message: Bytes) -> Result<Signature>;
	async fn verify(&self, message: Bytes, signature: Signature) -> Result<bool>;
}

/// An application which reads a stream of messages to either sign or verify.
pub struct Application {
	pub hsm: Box<dyn Hsm>,
	pub stream: Box<dyn ActionStream>,
}

/// The application implementation.
impl Application {
	pub async fn run(&mut self) -> Result<()> {
		while let Some(message) = self.stream.next().await {
			match message {
				Message::Sign(message) => {
					let signature = self.hsm.sign(message).await?;
					println!("Signed message: {:?}", signature);
				}
				Message::Verify(message, signature) => {
					let verified = self.hsm.verify(message, signature).await?;
					println!("Verified message: {:?}", verified);
				}
			}
		}
		Ok(())
	}
}
