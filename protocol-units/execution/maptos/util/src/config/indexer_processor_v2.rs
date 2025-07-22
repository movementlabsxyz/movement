use crate::config::common::default_maptos_indexer_grpc_ping_interval;

use super::common::{
	default_indexer_processor_auth_token, default_postgres_connection_string,
	default_indexer_processor_name,
	default_maptos_indexer_grpc_listen_hostname,
	default_maptos_indexer_grpc_listen_port,
};
use aptos_indexer_processor_sdk::aptos_indexer_transaction_stream::utils::additional_headers::AdditionalHeaders;
use aptos_indexer_processor_sdk::aptos_indexer_transaction_stream::TransactionStreamConfig;
use url::Url;
use serde::{Deserialize, Serialize};
use serde_json::{Value};
use crate::config::metrics_server::MetricsConfig;
use crate::config::health_server::Config as HealthServerConfig;
use processor_v2::config::indexer_processor_config::IndexerProcessorConfig;
use processor_v2::config::db_config::{DbConfig, PostgresConfig};
use processor_v2::config::processor_mode::{ProcessorMode, BootStrapConfig};

// Stream related configs.
const DEFAULT_INDEXER_GRPC_RECONNECTION_TIMEOUT_SECS: u64 = 30;
const DEFAULT_INDEXER_GRPC_RESPONSE_ITEM_TIMEOUT_SECS: u64 = 30;
const DEFAULT_INDEXER_GRPC_RECONNECTION_MAX_RETRIES: u64 = 10;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
	#[serde(default = "default_postgres_connection_string")]
	pub postgres_connection_string: String,

	#[serde(default = "default_indexer_processor_auth_token")]
	pub indexer_processor_auth_token: String,

	#[serde(default = "default_maptos_indexer_grpc_listen_hostname")]
	pub maptos_indexer_grpc_listen_hostname: String,

	#[serde(default = "default_maptos_indexer_grpc_listen_port")]
	pub maptos_indexer_grpc_listen_port: u16,

	#[serde(default = "default_indexer_processor_name")]
	pub processor_name: String,

	// This is to allow additional fields specified in the config file.
	pub additional_config: Value,

	pub metrics_config: MetricsConfig,

	pub health_config: HealthServerConfig,
}

impl Default for Config {
	fn default() -> Self {
		Self {
			processor_name: default_indexer_processor_name(),
			postgres_connection_string: default_postgres_connection_string(),
			indexer_processor_auth_token: default_indexer_processor_auth_token(),
			maptos_indexer_grpc_listen_hostname: default_maptos_indexer_grpc_listen_hostname(),
			maptos_indexer_grpc_listen_port: default_maptos_indexer_grpc_listen_port(),
			additional_config: Value::Null,
			metrics_config: MetricsConfig::default(),
			health_config: HealthServerConfig::default(),
		}
	}
}

impl Into<IndexerProcessorConfig> for Config {
	fn into(self) -> IndexerProcessorConfig {
		let indexer_grpc_data_service_address = self.get_grpc_url();
		let mut processor_config_map = serde_json::Map::new();
		// Required field.
		processor_config_map.insert("type".to_string(), Value::from(self.processor_name.clone()));
		
		match self.processor_name.as_str() {
			"default_processor" => {
				processor_config_map.insert("type".to_string(), Value::from(self.processor_name.clone()));
			},
			"ans_processor" => {
				processor_config_map.insert("ans_v1_primary_names_table_handle".to_string(), Value::from("temp"));
				processor_config_map.insert("ans_v1_name_records_table_handle".to_string(), Value::from("temp"));
				processor_config_map.insert("ans_v2_contract_address".to_string(), Value::from("0x67bf15b3eed0fc62deea9630bbbd1d48842550655140f913699a1ca7e6f727d8".to_string()));
			},
			"token_v2_processor" => {
				processor_config_map.insert("query_retries".to_string(), Value::from(5));
			},
			"token_processor" => {
				processor_config_map.insert("nft_points_contract".to_string(), Value::from("null"));
			},
			_ => {}
		}
		let processor_config = serde_json::from_value(Value::Object(processor_config_map)).unwrap();
		tracing::error!("processor_config: {:?}", processor_config);

		IndexerProcessorConfig {
			processor_config: processor_config,
			// Default running mode.
			processor_mode: ProcessorMode::Default(BootStrapConfig{
				initial_starting_version: 0,
			}),
			db_config: DbConfig::PostgresConfig(PostgresConfig {
				connection_string: self.postgres_connection_string,
				db_pool_size: PostgresConfig::default_db_pool_size(),
			}),
			transaction_stream_config: TransactionStreamConfig {
				indexer_grpc_data_service_address,
				auth_token: self.indexer_processor_auth_token,
				request_name_header: "MAPTOS_INDEXER_PROCESSOR".to_string(),
				additional_headers: AdditionalHeaders::default(),
				// Default to start from the genesis block.
				starting_version: Some(0),
				// Default to request all the blocks until the latest block.
				request_ending_version: None,
				indexer_grpc_http2_ping_interval_secs: default_maptos_indexer_grpc_ping_interval(),
				indexer_grpc_http2_ping_timeout_secs: default_maptos_indexer_grpc_ping_interval(),
				indexer_grpc_reconnection_timeout_secs: DEFAULT_INDEXER_GRPC_RECONNECTION_TIMEOUT_SECS,
				indexer_grpc_response_item_timeout_secs: DEFAULT_INDEXER_GRPC_RESPONSE_ITEM_TIMEOUT_SECS,
				indexer_grpc_reconnection_max_retries: DEFAULT_INDEXER_GRPC_RECONNECTION_MAX_RETRIES,
				transaction_filter: None,
			}
		}
	}
}

impl Config {
	fn get_grpc_url(&self) -> Url {
		let indexer_grpc_data_service_address = format!(
			"http://{}:{}",
			self.maptos_indexer_grpc_listen_hostname,
			self.maptos_indexer_grpc_listen_port
		);
		tracing::info!(
			"Connecting to indexer gRPC server at: {}",
			indexer_grpc_data_service_address.clone()
		);
		Url::parse(&indexer_grpc_data_service_address).unwrap()
	}
}
