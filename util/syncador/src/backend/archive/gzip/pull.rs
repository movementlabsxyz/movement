use crate::backend::archive::gzip::BUFFER_SIZE;
use crate::backend::PullOperations;
use crate::files::package::{Package, PackageElement};
use flate2::read::GzDecoder;
use std::collections::VecDeque;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::BufReader;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tar::Archive;
use tokio::{fs, task};
use tracing::info;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Candidate;

#[derive(Debug, Clone)]
pub struct Pull {
	pub destination_dir: PathBuf,
}

impl Pull {
	/// Creates a new Pull instance.
	pub fn new(destination_dir: PathBuf) -> Self {
		Self { destination_dir }
	}

	/// Iteratively collects all files (not directories) in the specified directory using BFS.
	async fn collect_files(dir: &Path, entries: &mut Vec<PathBuf>) -> Result<(), anyhow::Error> {
		let mut queue = VecDeque::new();
		queue.push_back(dir.to_path_buf());

		while let Some(current_dir) = queue.pop_front() {
			let mut read_dir = fs::read_dir(&current_dir).await?;

			while let Some(entry) = read_dir.next_entry().await? {
				let path = entry.path();
				if path.is_dir() {
					queue.push_back(path);
				} else {
					entries.push(path);
				}
			}
		}

		Ok(())
	}

	/// UnGZips and untars a manifest.
	async fn ungzip_tar_manifest(
		manifest: PackageElement,
		destination: PathBuf,
	) -> Result<PackageElement, anyhow::Error> {
		// Create the destination directory if it doesn't exist
		fs::create_dir_all(&destination).await?;

		println!("PULL manifest:{:?}", manifest);

		let mut unsplit_manifest =
			PackageElement { sync_files: vec![], root_dir: manifest.root_dir.clone() };

		// Unpack each archive in the manifest
		for (_relative_path, absolute_path) in manifest.try_path_tuples()? {
			// Recreate splited file if any
			let absolute_path = recreate_archive(absolute_path.to_path_buf())?;

			println!("PULL absolute_path {absolute_path:?}",);
			println!("PULL destination {destination:?}",);

			if !unsplit_manifest.sync_files.contains(&absolute_path) {
				unsplit_manifest.sync_files.push(absolute_path)
			}
		}

		println!("PULL unsplit_manifest:{:?}", unsplit_manifest);

		// Unpack each archive in the manifest
		for (_relative_path, absolute_path) in unsplit_manifest.try_path_tuples()? {
			let tar_gz = File::open(&absolute_path)?;
			let decoder = GzDecoder::new(tar_gz);
			let mut archive = Archive::new(decoder);

			// Extract the files to the destination directory
			let destination = destination.clone();
			task::spawn_blocking(move || archive.unpack(&destination)).await??;
		}

		// Create a new manifest based on the extracted contents
		let mut new_manifest = PackageElement::new(destination.clone());

		// Recursively add every file (not directory) in the destination directory to the new manifest
		let mut entries = Vec::new();
		Self::collect_files(&destination, &mut entries).await?;
		info!("Unarchived files: {:?}", entries.len());
		for file_path in entries {
			new_manifest.add_sync_file(file_path);
		}

		Ok(new_manifest)
	}
}

#[async_trait::async_trait]
impl PullOperations for Pull {
	async fn pull(&self, package: Option<Package>) -> Result<Option<Package>, anyhow::Error> {
		// If the package is None, return None
		info!("Archive pulling package: {:?}", package);
		if package.is_none() {
			return Ok(None);
		}

		let package = package.ok_or(anyhow::anyhow!("package is none"))?;

		let mut manifests = Vec::new();
		for manifest in package.0.into_iter() {
			let new_manifest =
				Self::ungzip_tar_manifest(manifest, self.destination_dir.clone()).await?;
			manifests.push(new_manifest);
		}
		Ok(Some(Package(manifests)))
	}
}

fn recreate_archive(archive_chunk: PathBuf) -> Result<PathBuf, anyhow::Error> {
	if archive_chunk
		.extension()
		.map(|ext| {
			println!("ext:{ext:?}",);
			ext != "chunk"
		})
		.unwrap_or(true)
	{
		//not a chunk file return.
		return Ok(archive_chunk);
	}

	let arhive_file_name = archive_chunk
		.file_name()
		.and_then(|file_name| file_name.to_str())
		.and_then(|file_name_str| file_name_str.strip_suffix(".chunk"))
		.and_then(|base_filename| {
			let base_filename_parts: Vec<&str> = base_filename.rsplitn(2, '_').collect();
			(base_filename_parts.len() > 1).then(|| base_filename_parts[1].to_string())
		})
		.ok_or(anyhow::anyhow!(format!(
			"Archive filename not found for chunk path:{:?}",
			archive_chunk.to_str()
		)))?;

	println!("PULL arhive_file_name:{:?}", arhive_file_name);

	let archive_path = archive_chunk.parent().map(|parent| parent.join(arhive_file_name)).ok_or(
		anyhow::anyhow!(format!(
			"Archive filename no root dir in path:{:?}",
			archive_chunk.to_str()
		)),
	)?;

	println!("PULL archive_path:{:?}", archive_path);
	let mut archive_file = OpenOptions::new()
		.create(true) // Create the file if it doesn't exist
		.append(true) // Open in append mode (do not overwrite)
		.open(&archive_path)?;

	let mut buffer = vec![0; BUFFER_SIZE];

	println!("PULL archive_chunk:{:?}", archive_chunk);
	let chunk_file = File::open(&archive_chunk)?;
	let mut chunk_reader = BufReader::new(chunk_file);

	loop {
		// Read a part of the chunk into the buffer
		let bytes_read = chunk_reader.read(&mut buffer)?;

		if bytes_read == 0 {
			break; // End of chunk file
		}

		// Write the buffer data to the output file
		archive_file.write_all(&buffer[..bytes_read])?;
	}

	let file_metadata = std::fs::metadata(&archive_path)?;
	let file_size = file_metadata.len() as usize;
	println!("PULL {archive_path:?} archive_chunk size: {file_size}",);

	Ok(archive_path)
}
