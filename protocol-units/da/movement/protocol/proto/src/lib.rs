pub mod v1beta2 {
	tonic::include_proto!("movementlabs.protocol_units.da.light_node.v1beta2"); // The string specified here
	pub const FILE_DESCRIPTOR_SET: &[u8] =
		tonic::include_file_descriptor_set!("movement-da-light-node-proto-descriptor");
}

// Re-export the latest version at the crate root
pub use v1beta2::*;
