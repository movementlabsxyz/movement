pub mod pull;
pub mod push;

pub(crate) const DEFAULT_CHUNK_SIZE: usize = 500 * 1024 * 1024; // 500 MB per chunk (adjustable)
pub(crate) const BUFFER_SIZE: usize = 10 * 1024 * 1024; // 10 MB buffer for each read/write operation

#[cfg(test)]
pub mod test {

	use crate::backend::archive::gzip::pull::Pull;
	use crate::backend::archive::gzip::push::Push;
	use crate::backend::PullOperations;
	use crate::backend::PushOperations;
	use crate::files::package::{Package, PackageElement};
	use std::fs::File;
	use std::io::BufWriter;
	use std::io::Write;
	use std::path::PathBuf;

	#[tokio::test]
	pub async fn test_archive_split() -> Result<(), anyhow::Error> {
		// 1) Chunk size is bigger than the archive. No split in chunk.
		process_archive_test("test_archive_split.tmp", 10 * 1024, 1024).await?;
		// 2) Chunk size is smaller than the archive. Several chunk is create and reconstructed.
		process_archive_test("test_archive_split2.tmp", 2024, 1024).await?;
		Ok(())
	}

	async fn process_archive_test(
		temp_file_name: &str,
		chunk_size: usize,
		buffer_size: usize,
	) -> Result<(), anyhow::Error> {
		//Create source and destination temp dir.
		let source_dir = tempfile::tempdir()?;
		let destination_dir = tempfile::tempdir()?;

		//1) First test file too small doesn't archive.

		let archive_file_path = source_dir.path().join(temp_file_name);
		{
			let file = File::create(&archive_file_path)?;
			let mut writer = BufWriter::new(file);
			//Fill with some data. 10 Mb
			let data: Vec<u8> = vec![2; 1024 * 1024];
			(0..10).try_for_each(|_| writer.write_all(&data))?;
		}

		let push = Push { archives_dir: source_dir.path().to_path_buf(), chunk_size, buffer_size };

		let element = PackageElement {
			sync_files: vec![archive_file_path],
			root_dir: source_dir.path().to_path_buf(),
		};
		let package = Package(vec![element]);
		let archive_package = push.push(package).await?;
		println!("TEST archive_package: {:?}", archive_package);

		let file_metadata = std::fs::metadata(&archive_package.0[0].sync_files[0])?;
		let file_size = file_metadata.len() as usize;
		println!("TEST Dest chunk file size: {file_size}",);

		// Unarchive and verify
		//move archive to dest folder.
		let dest_files = archive_package
			.0
			.into_iter()
			.flat_map(|element| element.sync_files)
			.map(|absolute_path| {
				let dest = destination_dir.path().join(absolute_path.file_name().unwrap());
				println!("TEST move file source:{absolute_path:?} dest:{dest:?}");
				std::fs::rename(&absolute_path, &dest)?;
				Ok(dest)
			})
			.collect::<std::io::Result<Vec<PathBuf>>>()?;

		let pull = Pull { destination_dir: destination_dir.path().to_path_buf() };
		let element = PackageElement {
			sync_files: dest_files,
			root_dir: destination_dir.path().to_path_buf(),
		};
		let package = Package(vec![element]);

		let dest_package = pull.pull(Some(package)).await;
		println!("ICICICIC dest_package: {:?}", dest_package);

		//verify the dest file has the right size
		let file_metadata = std::fs::metadata(&destination_dir.path().join(temp_file_name))?;
		let file_size = file_metadata.len() as usize;
		println!("Dest fiel size: {file_size}",);
		assert_eq!(file_size, 10 * 1024 * 1024, "dest file hasn't the right size: {file_size}");

		Ok(())
	}
}
