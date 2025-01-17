use anyhow::Context;
use celestia_types::{consts::appconsts::AppVersion, nmt::Namespace, Blob as CelestiaBlob};
use movement_da_util::blob::ir::blob::DaBlob;
use movement_signer::cryptography::Curve;
use serde::{Deserialize, Serialize};

/// Converts a [CelestiaBlob] into a [DaBlob].
pub fn into_da_blob<C>(blob: CelestiaBlob) -> Result<DaBlob<C>, anyhow::Error>
where
	C: Curve + for<'de> Deserialize<'de>,
{
	// decompress blob.data with zstd
	let decompressed =
		zstd::decode_all(blob.data.as_slice()).context("failed to decompress blob")?;

	// deserialize the decompressed data with bcs
	let blob = bcs::from_bytes(decompressed.as_slice()).context("failed to deserialize blob")?;

	Ok(blob)
}

pub struct CelestiaDaBlob<C>(pub DaBlob<C>, pub Namespace)
where
	C: Curve;

/// Tries to form a CelestiaBlob from a CelestiaDaBlob
impl<C> TryFrom<CelestiaDaBlob<C>> for CelestiaBlob
where
	C: Curve + Serialize,
{
	type Error = anyhow::Error;

	fn try_from(ir_blob: CelestiaDaBlob<C>) -> Result<Self, Self::Error> {
		// Extract the inner blob and namespace
		let CelestiaDaBlob(ir_blob, namespace) = ir_blob;

		// Serialize the inner blob with bcs
		let serialized_blob = bcs::to_bytes(&ir_blob).context("failed to serialize blob")?;

		// Compress the serialized data with zstd
		let compressed_blob =
			zstd::encode_all(serialized_blob.as_slice(), 0).context("failed to compress blob")?;

		// Construct the final CelestiaBlob by assigning the compressed data
		// and associating it with the provided namespace
		Ok(CelestiaBlob::new(namespace, compressed_blob, AppVersion::V2)
			.map_err(|e| anyhow::anyhow!(e))?)
	}
}
