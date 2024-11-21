extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use std::path::PathBuf;
use syn::{parse::Parse, parse_macro_input, punctuated::Punctuated, LitStr, Result, Token};

#[proc_macro]
pub fn cargo_workspace(_input: TokenStream) -> TokenStream {
	let workspace = buildtime_helpers::cargo::cargo_workspace().unwrap();
	let workspace_str = workspace.to_str().unwrap();
	let code = quote! {
		std::path::PathBuf::from(#workspace_str)
	};
	code.into()
}

#[proc_macro]
pub fn proto(_input: TokenStream) -> TokenStream {
	let proto = buildtime_helpers::proto::proto().unwrap();
	let proto_str = proto.to_str().unwrap();
	let code = quote! {
	   std::path::PathBuf::from(#proto_str)
	};
	code.into()
}

// Define a custom struct that holds a Punctuated list
struct ParsablePuncuated {
	list: Punctuated<LitStr, Token![,]>,
}

// Implement Parse for ParsablePuncuated
impl Parse for ParsablePuncuated {
	fn parse(input: syn::parse::ParseStream) -> Result<Self> {
		let list = Punctuated::parse_terminated(input)?;
		Ok(ParsablePuncuated { list })
	}
}

#[proc_macro]
pub fn proto_build_main(input: TokenStream) -> TokenStream {
	// Use custom parsing struct
	let ParsablePuncuated { list: inputs } = parse_macro_input!(input as ParsablePuncuated);

	// Assume proto_dir is provided by a runtime function and convert it to a string
	let proto_dir = buildtime_helpers::proto::proto().unwrap();
	let proto_dir_str = proto_dir.to_str().unwrap();

	// Collect input files into a Rust array expression
	let proto_files: Vec<_> = inputs
		.iter()
		.map(|lit_str| {
			let file = lit_str.value();
			// Combine proto_dir with the relative path
			let full_path = PathBuf::from(proto_dir_str).join(file).display().to_string();
			quote! { #full_path }
		})
		.collect();

	// Generate the code
	let expanded = quote! {
		fn main() -> Result<(), Box<dyn std::error::Error>> {
			let proto_files = &[#(#proto_files),*];
			let proto_include_dirs = &[#proto_dir_str];

			// Set up file descriptors for reflection
			let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
			let crate_name = std::env::var("CARGO_PKG_NAME").unwrap();
			let proto_descriptor_filename = format!("{}-descriptor.bin", crate_name);
			let descriptor_file_path = out_dir.join(proto_descriptor_filename);

			// Check if specific features are enabled and default to enabling both if neither is enabled
			let client_enabled = cfg!(feature = "client");
			let server_enabled = cfg!(feature = "server");

			let mut config = tonic_build::configure()
			.file_descriptor_set_path(descriptor_file_path)
			.include_file("all.rs")
			.build_client(client_enabled)
			.build_server(server_enabled);


			// Compile the proto files based on the configuration
			config.compile_protos(proto_files, proto_include_dirs)?;

			Ok(())
		}
	};

	// Convert the generated code back into a token stream
	TokenStream::from(expanded)
}
