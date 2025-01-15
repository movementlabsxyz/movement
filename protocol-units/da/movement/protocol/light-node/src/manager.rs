use super::{LightNode, LightNodeRuntime};
use ecdsa::{
	elliptic_curve::{
		generic_array::ArrayLength,
		ops::Invert,
		point::PointCompression,
		sec1::{FromEncodedPoint, ModulusSize, ToEncodedPoint},
		subtle::CtOption,
		AffinePoint, CurveArithmetic, FieldBytesSize, PrimeCurve, Scalar,
	},
	hazmat::{DigestPrimitive, SignPrimitive, VerifyPrimitive},
	SignatureSize,
};
use godfig::{backend::config_file::ConfigFile, Godfig};
use movement_da_light_node_celestia::da::Da as CelestiaDa;
use movement_da_light_node_digest_store::da::Da as DigestStoreDa;
use movement_da_light_node_verifier::signed::InKnownSignersVerifier;
use movement_da_util::config::Config;

#[derive(Clone)]
pub struct Manager<LightNode>
where
	LightNode: LightNodeRuntime,
{
	godfig: Godfig<Config, ConfigFile>,
	_marker: std::marker::PhantomData<LightNode>,
}

// Implements a very simple manager using a marker strategy pattern.
impl<C> Manager<LightNode<C, DigestStoreDa<CelestiaDa>, InKnownSignersVerifier<C>>>
where
	C: PrimeCurve + CurveArithmetic + DigestPrimitive + PointCompression,
	Scalar<C>: Invert<Output = CtOption<Scalar<C>>> + SignPrimitive<C>,
	SignatureSize<C>: ArrayLength<u8>,
	AffinePoint<C>: FromEncodedPoint<C> + ToEncodedPoint<C> + VerifyPrimitive<C>,
	FieldBytesSize<C>: ModulusSize,
{
	pub async fn new(file: tokio::fs::File) -> Result<Self, anyhow::Error> {
		let godfig = Godfig::new(
			ConfigFile::new(file),
			vec![
				"celestia_da_light_node_config".to_string(), // in this example this comes from the structuring of the config file
			],
		);
		Ok(Self { godfig, _marker: std::marker::PhantomData })
	}

	pub async fn try_light_node(
		&self,
	) -> Result<LightNode<C, DigestStoreDa<CelestiaDa>, InKnownSignersVerifier<C>>, anyhow::Error>
	where
		C: PrimeCurve + CurveArithmetic + DigestPrimitive + PointCompression,
		Scalar<C>: Invert<Output = CtOption<Scalar<C>>> + SignPrimitive<C>,
		SignatureSize<C>: ArrayLength<u8>,
		AffinePoint<C>: FromEncodedPoint<C> + ToEncodedPoint<C>,
		FieldBytesSize<C>: ModulusSize,
	{
		let config = self.godfig.try_wait_for_ready().await?;
		LightNode::try_from_config(config).await
	}

	pub async fn try_run(&self) -> Result<(), anyhow::Error> {
		let light_node = self.try_light_node().await?;
		light_node.run().await
	}
}
