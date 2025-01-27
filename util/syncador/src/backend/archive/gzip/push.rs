use crate::backend::archive::gzip::BUFFER_SIZE;
use crate::backend::archive::gzip::DEFAULT_CHUNK_SIZE;
use crate::backend::PushOperations;
use crate::files::package::{Package, PackageElement};
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::Path;
use std::path::PathBuf;
use tar::Builder;

#[derive(Debug, Clone)]
pub struct Push {
	pub archives_dir: PathBuf,
	pub chunk_size: usize,
	pub buffer_size: usize,
}

impl Push {
	pub fn new(archives_dir: PathBuf) -> Self {
		Self { archives_dir, chunk_size: DEFAULT_CHUNK_SIZE, buffer_size: BUFFER_SIZE }
	}

	/// Tar GZips a manifest.
	fn tar_gzip_manifest(
		manifest: PackageElement,
		destination: PathBuf,
		root_dir: PathBuf,
		chunk_size: usize,
		buffer_size: usize,
	) -> Result<PackageElement, anyhow::Error> {
		// create the archive builder
		let file = File::create(destination.clone())?;
		{
			let encoder = GzEncoder::new(file, Compression::default());
			let mut tar_builder = Builder::new(encoder);

			for (relative_path, absolute_path) in manifest.try_path_tuples()? {
				let file = &mut std::fs::File::open(absolute_path)?;
				tar_builder.append_file(relative_path, file)?;
			}

			// Finish writing the tar archive
			tar_builder.finish()?;
		}

		// Split the archive if needed
		let destinations = split_archive(destination, &root_dir, chunk_size, buffer_size)?;
		let mut new_manifest = PackageElement::new(root_dir);
		for dest in destinations {
			new_manifest.add_sync_file(dest);
		}
		Ok(new_manifest)
	}
}

#[async_trait::async_trait]
impl PushOperations for Push {
	async fn push(&self, package: Package) -> Result<Package, anyhow::Error> {
		let mut manifests = Vec::new();
		for (i, manifest) in package.0.into_iter().enumerate() {
			let new_manifest = tokio::task::spawn_blocking({
				let archive_dir = self.archives_dir.clone();
				let chunk_size = self.chunk_size;
				let buffer_size = self.buffer_size;

				move || {
					Self::tar_gzip_manifest(
						manifest,
						archive_dir.join(format!("{}.tar.gz", i)),
						archive_dir,
						chunk_size,
						buffer_size,
					)
				}
			})
			.await??;
			manifests.push(new_manifest);
		}
		Ok(Package(manifests))
	}
}

fn split_archive<P: AsRef<Path>>(
	archive: PathBuf,
	root_dir: P,
	chunk_size: usize,
	buffer_size: usize,
) -> Result<Vec<PathBuf>, anyhow::Error> {
	let output_dir = root_dir.as_ref();

	// Check the file size before proceeding with the split
	if file_size <= chunk_size {
		return Ok(vec![archive]);
	}

	let archive_file = File::open(&archive)?;
	std::fs::create_dir_all(output_dir)?;

	let mut chunk_num = 0;
	let mut buffer = vec![0; buffer_size];

	let archive_relative_path = archive.strip_prefix(&output_dir)?;
	let mut input_reader = BufReader::new(archive_file);

	let mut chunk_list = vec![];
	loop {
		// Create a new file for the chunk
		let chunk_path = output_dir.join(format!(
			"{}_{:03}.chunk",
			archive_relative_path.to_string_lossy(),
			chunk_num
		));

		let mut chunk_file = File::create(&chunk_path)?;

		let mut all_read_bytes = 0;
		let end = loop {
			// Read a part of the chunk into the buffer
			let bytes_read = input_reader.read(&mut buffer)?;
			if bytes_read == 0 {
				break true; // End of chunk file
			}

			// Write the buffer data to the output file
			chunk_file.write_all(&buffer[..bytes_read])?;
			all_read_bytes += bytes_read;
			if all_read_bytes >= chunk_size {
				break false;
			}
		};

		if all_read_bytes == 0 {
			break; // End of chunk file and discard the current one.
		}

		let file_metadata = std::fs::metadata(&chunk_path)?;
		let file_size = file_metadata.len() as usize;
		println!("PUSH {chunk_path:?} chunk_file size: {file_size}",);

		chunk_num += 1;
		chunk_list.push(chunk_path);
		if end {
			break; // End of chunk file
		}
	}

	Ok(chunk_list)
}
