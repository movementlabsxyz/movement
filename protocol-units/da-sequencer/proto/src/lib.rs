pub mod v1 {
	tonic::include_proto!("movementlabs.protocol_units.da.sequencer.v1"); // The string specified here
	pub const FILE_DESCRIPTOR_SET: &[u8] =
		tonic::include_file_descriptor_set!("movement-da-sequencer-proto-descriptor");
}

// Re-export the latest version at the crate root
pub use v1::*;
