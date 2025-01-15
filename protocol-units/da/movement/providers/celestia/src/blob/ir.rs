use anyhow::Context;
use celestia_types::{consts::appconsts::AppVersion, nmt::Namespace, Blob as CelestiaBlob};
use movement_da_util::blob::ir::blob::DaBlob;

/// Converts a [CelestiaBlob] into a [DaBlob].
pub fn into_da_blob(blob: CelestiaBlob) -> Result<DaBlob, anyhow::Error> {
	// decompress blob.data with zstd
	let decompressed =
		zstd::decode_all(blob.data.as_slice()).context("failed to decompress blob")?;

	// deserialize the decompressed data with bcs
	let blob = bcs::from_bytes(decompressed.as_slice()).context("failed to deserialize blob")?;

	Ok(blob)
}

pub struct CelestiaDaBlob(pub DaBlob, pub Namespace);

/// Tries to form a CelestiaBlob from a CelestiaDaBlob
impl TryFrom<CelestiaDaBlob> for CelestiaBlob {
	type Error = anyhow::Error;

	fn try_from(ir_blob: CelestiaDaBlob) -> Result<Self, Self::Error> {
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
