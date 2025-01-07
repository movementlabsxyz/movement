pub mod action_stream;
pub mod cli;
pub mod cryptography;
pub mod server;
use movement_signer::{cryptography::Curve, Signer, Signing};

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

/// An application which reads a stream of messages to either sign or verify.
pub struct Application<O, C>
where
	O: Signing<C>,
	C: Curve,
{
	hsm: Signer<O, C>,
	stream: Box<dyn ActionStream>,
}

/// The application implementation.
impl<O, C> Application<O, C>
where
	O: Signing<C>,
	C: Curve,
{
	/// Creates a new application.
	pub fn new(hsm: Signer<O, C>, stream: Box<dyn ActionStream>) -> Self {
		Self { hsm, stream }
	}

	/// Runs the application.
	pub async fn run(&mut self) -> Result<(), anyhow::Error> {
		while let Some(message) = self.stream.next().await? {
			println!("RECEIVED: {:?}", message);
			match message {
				Message::Sign(message) => {
					println!("SIGNING: {:?}", message);
					let signature = self.hsm.sign(message.0.as_slice()).await?;
					let public_key = self.hsm.public_key().await?;
					println!("SIGNED:\n{:?}\n{:?}\n{:?}", message, public_key, signature);
					// todo: reintroduce this if you want to no
					// self.stream.notify(Message::Verify(message, public_key, signature)).await?;
				}
				Message::Verify(message, public_key, signature) => {
					println!("VERIFYING:\n{:?}\n{:?}\n{:?}", message, public_key, signature);
					println!("VERIFIED");
				}
			}
		}
		Ok(())
	}
}
