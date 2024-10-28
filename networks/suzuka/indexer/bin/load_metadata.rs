use anyhow::{anyhow, Context, Result};
use reqwest::Url;
use tracing::info;
use tracing_subscriber::EnvFilter;

const HASURA_METADATA: &str = include_str!("../hasura_metadata.json");

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	let indexer_api_url =
		std::env::var("INDEXER_API_URL").unwrap_or("http://127.0.0.1:8085".to_string());

	// Replace the postgres connection definition in the metadata file
	// with the provided in the env var INDEXER_V2_POSTGRES_URL
	let postgres_host = std::env::var("POSTGRES_DB_HOST").unwrap_or("postgres".to_string());
	let postgres_url = format!("postgres://postgres:password@{postgres_host}:5432/postgres");
	let metadata_file = HASURA_METADATA.replace("INDEXER_V2_POSTGRES_URL", &postgres_url);

	post_metadata(indexer_api_url.parse()?, &metadata_file)
		.await
		.context("Failed to apply Hasura metadata for Indexer API")?;

	Ok(())
}

/// This submits a POST request to apply metadata to a Hasura API.
async fn post_metadata(url: Url, metadata_content: &str) -> Result<()> {
	// Parse the metadata content as JSON.
	let metadata_json: serde_json::Value = serde_json::from_str(metadata_content)?;

	// Make the request.
	info!("Submitting request to apply Hasura metadata");
	let response =
		make_hasura_metadata_request(url, "replace_metadata", Some(metadata_json)).await?;
	info!("Received response for applying Hasura metadata: {:?}", response);

	// Confirm that the metadata was applied successfully and there is no inconsistency
	// between the schema and the underlying DB schema.
	if let Some(obj) = response.as_object() {
		if let Some(is_consistent_val) = obj.get("is_consistent") {
			if is_consistent_val.as_bool() == Some(true) {
				return Ok(());
			}
		}
	}

	Err(anyhow!(
        "Something went wrong applying the Hasura metadata, perhaps it is not consistent with the DB. Response: {:#?}",
        response
    ))
}

/// This confirms that the metadata has been applied. We use this in the health
/// checker.
pub async fn confirm_metadata_applied(url: Url) -> Result<()> {
	// Make the request.
	info!("Confirming Hasura metadata applied...");
	let response = make_hasura_metadata_request(url, "export_metadata", None).await?;
	info!("Received response for confirming Hasura metadata applied: {:?}", response);

	// If the sources field is set it means the metadata was applied successfully.
	if let Some(obj) = response.as_object() {
		if let Some(sources) = obj.get("sources") {
			if let Some(sources) = sources.as_array() {
				if !sources.is_empty() {
					return Ok(());
				}
			}
		}
	}

	Err(anyhow!("The Hasura metadata has not been applied yet. Response: {:#?}", response))
}

/// The /v1/metadata endpoint supports a few different operations based on the `type`
/// field in the request body. All requests have a similar format, with these `type`
/// and `args` fields.
async fn make_hasura_metadata_request(
	mut url: Url,
	typ: &str,
	args: Option<serde_json::Value>,
) -> Result<serde_json::Value> {
	let client = reqwest::Client::new();

	// Update the query path.
	url.set_path("/v1/metadata");

	// Construct the payload.
	let mut payload = serde_json::Map::new();
	payload.insert("type".to_string(), serde_json::Value::String(typ.to_string()));

	// If args is provided, use that. Otherwise use an empty object. We have to set it
	// no matter what because the API expects the args key to be set.
	let args = match args {
		Some(args) => args,
		None => serde_json::Value::Object(serde_json::Map::new()),
	};
	payload.insert("args".to_string(), args);

	// Send the POST request.
	let response = if let Ok(auth_key) = std::env::var("HASURA_ADMIN_AUTH_KEY") {
		client
			.post(url)
			.header("X-Hasura-Admin-Secret", auth_key)
			.json(&payload)
			.send()
			.await?
	} else {
		client.post(url).json(&payload).send().await?
	};

	// Return the response as a JSON value.
	response.json().await.context("Failed to parse response as JSON")
}
