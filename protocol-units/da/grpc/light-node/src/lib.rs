pub mod v1beta1 {
	tonic::include_proto!("movementlabs.protocol_units.da.light_node.v1beta1"); // The string specified here
	pub const FILE_DESCRIPTOR_SET: &[u8] =
		tonic::include_file_descriptor_set!("movement-da-light-node-grpc-descriptor");
}

// Re-export the latest version at the crate root
pub use v1beta1::*;
