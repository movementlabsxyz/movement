use tokio::fs;

pub async fn make_parent_dirs(path: &str) -> Result<(), anyhow::Error> {
	let parent = std::path::Path::new(path)
		.parent()
		.ok_or(anyhow::anyhow!("Failed to get parent directory."))?;
	fs::create_dir_all(parent).await?;
	Ok(())
}
