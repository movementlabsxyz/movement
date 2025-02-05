use movement_signer::{cryptography::Curve, Signing};

// A signer simply wraps a [movement_signer::Signer] instance.
pub struct Signer<O, C>(movement_signer::Signer<O, C>)
where
	O: Signing<C>,
	C: Curve;

impl<O, C> Signer<O, C>
where
	O: Signing<C>,
	C: Curve,
{
	/// Creates a new [Signer] instance.
	pub fn new(provider: O) -> Self {
		Self(movement_signer::Signer::new(provider))
	}

	/// Returns a reference to the inner [movement_signer::Signer] instance.
	pub fn inner(&self) -> &movement_signer::Signer<O, C> {
		&self.0
	}

	/// Converts the [Signer] instance into the inner [movement_signer::Signer] instance.
	pub fn into_inner(self) -> O {
		self.0.into_inner()
	}
}
