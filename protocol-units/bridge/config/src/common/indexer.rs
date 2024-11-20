use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
	/// URL for the bridge indexer database
	#[serde(default = "default_database_url")]
	pub indexer_url: String,
}

impl Default for DatabaseConfig {
	fn default() -> Self {
		Self { indexer_url: default_database_url() }
	}
}

fn default_database_url() -> String {
	"postgresql://postgres:password@localhost:5432".to_string()
}
