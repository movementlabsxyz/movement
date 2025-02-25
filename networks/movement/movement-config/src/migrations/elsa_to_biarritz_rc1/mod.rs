pub mod dot_movement;

use crate::releases::biarritz_rc1::Config;
use movement_signer_loader::identifiers::{local::Local, SignerIdentifier};
use std::future::Future;
use tracing::info;

/// A struct for managing the Elsa to biarritz rc1 migration.
///
/// This migration was written before we used versioned config scripts, so it is performed over raw JSON values.
///
/// Example JSON:
/*```json
{
  "maptos_config": {
	"chain": {
	  "maptos_chain_id": 126,
	  "maptos_rest_listen_hostname": "0.0.0.0",
	  "maptos_rest_listen_port": 30731,
	  "maptos_private_key": "0x<redacted-hex-string>",
	  "maptos_read_only": false,
	  "enabled_pruning": false,
	  "maptos_ledger_prune_window": 50000000,
	  "maptos_epoch_snapshot_prune_window": 50000000,
	  "maptos_state_merkle_prune_window": 100000,
	  "maptos_db_path": "/.movement/maptos/126/.maptos",
	  "genesis_timestamp_microseconds": 1600000000000,
	  "genesis_block_hash_hex": "25112f5405bbc65b2542a67d94094f12f4d2e287025480efcdb6200c5fed8671"
	},
	"indexer": {
	  "maptos_indexer_grpc_listen_hostname": "0.0.0.0",
	  "maptos_indexer_grpc_listen_port": 30734,
	  "maptos_indexer_grpc_inactivity_timeout": 60,
	  "maptos_indexer_grpc_inactivity_ping_interval": 10,
	  "maptos_indexer_grpc_healthcheck_hostname": "0.0.0.0",
	  "maptos_indexer_grpc_healthcheck_port": 8084
	},
	"indexer_processor": {
	  "postgres_connection_string": "postgres://postgres:password@postgres:5432/postgres",
	  "indexer_processor_auth_token": "auth_token"
	},
	"client": {
	  "maptos_rest_connection_hostname": "movement-full-node",
	  "maptos_rest_connection_port": 30731,
	  "maptos_faucet_rest_connection_hostname": "movement-faucet-service",
	  "maptos_faucet_rest_connection_port": 30732,
	  "maptos_indexer_grpc_connection_hostname": "0.0.0.0",
	  "maptos_indexer_grpc_connection_port": 30734
	},
	"faucet": {
	  "maptos_rest_connection_hostname": "movement-full-node",
	  "maptos_rest_connection_port": 30731,
	  "maptos_faucet_rest_listen_hostname": "0.0.0.0",
	  "maptos_faucet_rest_listen_port": 30732
	},
	"fin": {
	  "fin_rest_listen_hostname": "0.0.0.0",
	  "fin_rest_listen_port": 30733
	},
	"load_shedding": {
	  "max_transactions_in_flight": null
	},
	"mempool": {
	  "sequence_number_ttl_ms": 180000,
	  "gc_slot_duration_ms": 2000
	},
	"access_control": {
	  "ingress_account_whitelist": null
	}
  },
  "celestia_da_light_node_config": {
	"Local": {
	  "appd": {
		"celestia_rpc_listen_hostname": "0.0.0.0",
		"celestia_rpc_listen_port": 26657,
		"celestia_websocket_connection_protocol": "ws",
		"celestia_websocket_connection_hostname": "movement-celestia-bridge",
		"celestia_websocket_connection_port": 26658,
		"celestia_auth_token": "<redacted-auth-token>",
		"celestia_chain_id": "d65d2d9df7f98e7df32c",
		"celestia_namespace": "AAAAAAAAAAAAAAAAAAAAAAAAAPsKWni5iYCS1KE=",
		"celestia_path": "/.movement/celestia/d65d2d9df7f98e7df32c/.celestia-app",
		"celestia_validator_address": "celestia18j6sgshcppptsk6e0qgqkj52jj7ktsflcxnr5d",
		"celestia_appd_use_replace_args": false,
		"celestia_appd_replace_args": []
	  },
	  "bridge": {
		"celestia_rpc_connection_protocol": "http",
		"celestia_rpc_connection_hostname": "movement-celestia-appd",
		"celestia_rpc_connection_port": 26657,
		"celestia_websocket_listen_hostname": "0.0.0.0",
		"celestia_websocket_listen_port": 26658,
		"celestia_bridge_path": "/.movement/celestia/d65d2d9df7f98e7df32c/.celestia-node",
		"celestia_bridge_use_replace_args": false,
		"celestia_bridge_replace_args": []
	  },
	  "da_light_node": {
		"celestia_rpc_connection_protocol": "http",
		"celestia_rpc_connection_hostname": "movement-celestia-appd",
		"celestia_rpc_connection_port": 26657,
		"celestia_websocket_connection_hostname": "movement-celestia-bridge",
		"celestia_websocket_connection_port": 26658,
		"movement_da_light_node_listen_hostname": "0.0.0.0",
		"movement_da_light_node_listen_port": 30730,
		"movement_da_light_node_connection_protocol": "http",
		"movement_da_light_node_connection_hostname": "movement-celestia-da-light-node",
		"movement_da_light_node_connection_port": 30730,
		"movement_da_light_node_http1": false,
		"da_signers": {
		  "private_key_hex": "<redacted-hex-string>",
		  "public_keys_hex": [
			"026df08227f565470ed01dbf56cde3fe5f66cc8ef793088cf68329bdd23a5d1f28"
		  ]
		}
	  },
	  "celestia_force_new_chain": true,
	  "memseq": {
		"sequencer_chain_id": "d65d2d9df7f98e7df32c",
		"sequencer_database_path": "/.movement/memseq/d65d2d9df7f98e7df32c/.memseq",
		"memseq_build_time": 1000,
		"memseq_max_block_size": 2048
	  },
	  "da_light_node_is_initial": false,
	  "access_control": {
		"ingress_account_whitelist": null
	  },
	  "digest_store": {
		"digest_store_db_path": "/tmp/digest_store_db"
	  }
	}
  },
  "mcr": {
	"eth_connection": {
	  "eth_rpc_connection_protocol": "http",
	  "eth_rpc_connection_hostname": "setup",
	  "eth_rpc_connection_port": 8090,
	  "eth_ws_connection_protocol": "ws",
	  "eth_ws_connection_hostname": "setup",
	  "eth_ws_connection_port": 8090,
	  "eth_chain_id": 3073
	},
	"settle": {
	  "should_settle": false,
	  "signer_private_key": "<redacted-hex-string>",
	  "mcr_contract_address": "0x5fc8d32690cc91d4c39d9d3abcbd16989f875707",
	  "settlement_super_block_size": 1,
	  "settlement_admin_mode": false
	},
	"transactions": {
	  "gas_limit": 10000000000000000,
	  "batch_timeout": 2000,
	  "transaction_send_retries": 10
	},
	"maybe_run_local": true,
	"deploy": {
	  "mcr_deployment_working_directory": "protocol-units/settlement/mcr/contracts",
	  "mcr_deployment_account_private_key": "<redacted-hex-string>"
	},
	"testing": null
  },
  "da_db": {
	"da_db_path": "/.movement/movement-da-db"
  },
  "execution_extension": {
	"block_retry_count": 10,
	"block_retry_increment_microseconds": 5000
  },
  "syncing": {
	"movement_sync": "leader::mainnet-l-sync-bucket-sync<=>{default_signer_address_whitelist,maptos,maptos-storage,movement-da-db}/**",
	"application_id": [
	  26,
	  43,
	  60,
	  77,
	  94,
	  111,
	  122,
	  139,
	  156,
	  173,
	  190,
	  207,
	  208,
	  225,
	  242,
	  3,
	  20,
	  37,
	  54,
	  71,
	  88,
	  105,
	  122,
	  139,
	  156,
	  173,
	  190,
	  207,
	  208,
	  225,
	  242,
	  3
	],
	"syncer_id": [
	  157,
	  176,
	  168,
	  220,
	  173,
	  5,
	  115,
	  141,
	  114,
	  148,
	  25,
	  118,
	  79,
	  232,
	  97,
	  122,
	  208,
	  240,
	  178,
	  127,
	  137,
	  16,
	  141,
	  102,
	  130,
	  237,
	  209,
	  161,
	  166,
	  182,
	  69,
	  205
	],
	"root_dir": "/.movement"
  }
}
```
 */
 */
pub struct ElsaToBiarritzRc1;

/// Errors thrown by ElsaToBiarritzRc1 migrations.
#[derive(Debug, thiserror::Error)]
pub enum ElsaToBiarritzRc1Error {
	#[error("migration failed: {0}")]
	MigrationFailed(#[source] Box<dyn std::error::Error + Send + Sync>),
}

impl ElsaToBiarritzRc1 {
	/// Migrates from the Elsa config to the Biarritz RC1 config.
	pub fn migrate(json: serde_json::Value) -> Result<Config, ElsaToBiarritzRc1Error> {
		let mut migrated_json = json.clone();

		// Helper function to extract a private key from a nested path in the JSON
		fn extract_private_key(
			json: &serde_json::Value,
			path: &[&str],
		) -> Result<String, ElsaToBiarritzRc1Error> {
			let full_string = path
				.iter()
				.fold(Some(json), |acc, key| acc.and_then(|j| j.get(key)))
				.and_then(|key| key.as_str())
				.map(|s| s.to_string())
				.ok_or_else(|| {
					ElsaToBiarritzRc1Error::MigrationFailed(
						format!("Path {:?} not found or invalid", path).into(),
					)
				});

			// remove the 0x prefix if it exists
			full_string.map(|s| s.strip_prefix("0x").unwrap_or(&s).to_string())
		}

		// Helper function to replace a private key with a signer identifier in the JSON
		fn replace_with_signer_identifier(
			migrated_json: &mut serde_json::Value,
			path: &[&str],
			new_field: &str,
			signer_identifier: SignerIdentifier,
		) -> Result<(), ElsaToBiarritzRc1Error> {
			// Navigate to the parent object of the target field
			let field_to_modify = path[..path.len() - 1]
				.iter()
				.fold(Some(migrated_json), |acc, key| acc.and_then(|j| j.get_mut(key)));

			if let Some(parent) = field_to_modify.and_then(|j| j.as_object_mut()) {
				// Preserve the parent object and modify only the target field
				if let Some(last_key) = path.last() {
					info!("Replacing {:?} with {:?}", last_key, new_field);
					parent.remove(*last_key); // Remove the specific field
					parent.insert(
						new_field.to_string(), // Add the new field
						serde_json::to_value(signer_identifier).unwrap_or_default(),
					);
				} else {
					return Err(ElsaToBiarritzRc1Error::MigrationFailed(
						format!("Path {:?} is invalid or missing", path).into(),
					));
				}

				Ok(())
			} else {
				Err(ElsaToBiarritzRc1Error::MigrationFailed(
					format!("Parent object for path {:?} not found", path).into(),
				))
			}
		}

		// Define the fields to migrate
		let migrations = vec![
			(
				vec!["maptos_config", "chain", "maptos_private_key"],
				"maptos_private_key_signer_identifier",
			),
			(
				vec![
					"celestia_da_light_node_config",
					"Local",
					"da_light_node",
					"da_signers",
					"private_key_hex",
				],
				"signer_identifier",
			),
			(vec!["mcr", "settle", "signer_private_key"], "signer_identifier"),
			(vec!["mcr", "deploy", "mcr_deployment_account_private_key"], "signer_identifier"),
		];

		// Perform each migration
		for (private_key_path, new_field) in migrations {
			let private_key = extract_private_key(&json, &private_key_path)?;
			let signer_identifier =
				SignerIdentifier::Local(Local { private_key_hex_bytes: private_key });
			replace_with_signer_identifier(
				&mut migrated_json,
				&private_key_path,
				new_field,
				signer_identifier,
			)?;
		}

		info!("{:#?}", migrated_json);

		// Deserialize the JSON into a Config struct, filling in the missing fields with defaults
		let config: Config = serde_json::from_value(migrated_json)
			.map_err(|e| ElsaToBiarritzRc1Error::MigrationFailed(e.into()))?;

		Ok(config)
	}
}

pub trait MigrateElsaToBiarritzRc1 {
	/// Handles all side effects of the migration including writing to file and outputs a copy of the updated config.
	fn migrate_elsa_to_biarritz_rc1(
		&self,
	) -> impl Future<Output = Result<Config, ElsaToBiarritzRc1Error>>;
}

#[cfg(test)]
mod tests {
	use super::*;
	use tracing::info;

	#[test]
	#[tracing_test::traced_test]
	fn test_migration() -> Result<(), anyhow::Error> {
		let json = serde_json::json!({
				"maptos_config": {
				  "chain": {
					"maptos_chain_id": 126,
					"maptos_rest_listen_hostname": "0.0.0.0",
					"maptos_rest_listen_port": 30731,
					"maptos_private_key": "0x<redacted-hex-string>",
					"maptos_read_only": false,
					"enabled_pruning": false,
					"maptos_ledger_prune_window": 50000000,
					"maptos_epoch_snapshot_prune_window": 50000000,
					"maptos_state_merkle_prune_window": 100000,
					"maptos_db_path": "/.movement/maptos/126/.maptos",
					"genesis_timestamp_microseconds": 1600000000000 as i64,
					"genesis_block_hash_hex": "25112f5405bbc65b2542a67d94094f12f4d2e287025480efcdb6200c5fed8671"
				  },
				  "indexer": {
					"maptos_indexer_grpc_listen_hostname": "0.0.0.0",
					"maptos_indexer_grpc_listen_port": 30734,
					"maptos_indexer_grpc_inactivity_timeout": 60,
					"maptos_indexer_grpc_inactivity_ping_interval": 10,
					"maptos_indexer_grpc_healthcheck_hostname": "0.0.0.0",
					"maptos_indexer_grpc_healthcheck_port": 8084
				  },
				  "indexer_processor": {
					"postgres_connection_string": "postgres://postgres:password@postgres:5432/postgres",
					"indexer_processor_auth_token": "auth_token"
				  },
				  "client": {
					"maptos_rest_connection_hostname": "movement-full-node",
					"maptos_rest_connection_port": 30731,
					"maptos_faucet_rest_connection_hostname": "movement-faucet-service",
					"maptos_faucet_rest_connection_port": 30732,
					"maptos_indexer_grpc_connection_hostname": "0.0.0.0",
					"maptos_indexer_grpc_connection_port": 30734
				  },
				  "faucet": {
					"maptos_rest_connection_hostname": "movement-full-node",
					"maptos_rest_connection_port": 30731,
					"maptos_faucet_rest_listen_hostname": "0.0.0.0",
					"maptos_faucet_rest_listen_port": 30732
				  },
				  "fin": {
					"fin_rest_listen_hostname": "0.0.0.0",
					"fin_rest_listen_port": 30733
				  },
				  "load_shedding": {
					"max_transactions_in_flight": null
				  },
				  "mempool": {
					"sequence_number_ttl_ms": 180000,
					"gc_slot_duration_ms": 2000
				  },
				  "access_control": {
					"ingress_account_whitelist": null
				  }
				},
				"celestia_da_light_node_config": {
				  "Local": {
					"appd": {
					  "celestia_rpc_listen_hostname": "0.0.0.0",
					  "celestia_rpc_listen_port": 26657,
					  "celestia_websocket_connection_protocol": "ws",
					  "celestia_websocket_connection_hostname": "movement-celestia-bridge",
					  "celestia_websocket_connection_port": 26658,
					  "celestia_auth_token": "<redacted-auth-token>",
					  "celestia_chain_id": "d65d2d9df7f98e7df32c",
					  "celestia_namespace": "AAAAAAAAAAAAAAAAAAAAAAAAAPsKWni5iYCS1KE=",
					  "celestia_path": "/.movement/celestia/d65d2d9df7f98e7df32c/.celestia-app",
					  "celestia_validator_address": "celestia18j6sgshcppptsk6e0qgqkj52jj7ktsflcxnr5d",
					  "celestia_appd_use_replace_args": false,
					  "celestia_appd_replace_args": []
					},
					"bridge": {
					  "celestia_rpc_connection_protocol": "http",
					  "celestia_rpc_connection_hostname": "movement-celestia-appd",
					  "celestia_rpc_connection_port": 26657,
					  "celestia_websocket_listen_hostname": "0.0.0.0",
					  "celestia_websocket_listen_port": 26658,
					  "celestia_bridge_path": "/.movement/celestia/d65d2d9df7f98e7df32c/.celestia-node",
					  "celestia_bridge_use_replace_args": false,
					  "celestia_bridge_replace_args": []
					},
					"da_light_node": {
					  "celestia_rpc_connection_protocol": "http",
					  "celestia_rpc_connection_hostname": "movement-celestia-appd",
					  "celestia_rpc_connection_port": 26657,
					  "celestia_websocket_connection_hostname": "movement-celestia-bridge",
					  "celestia_websocket_connection_port": 26658,
					  "movement_da_light_node_listen_hostname": "0.0.0.0",
					  "movement_da_light_node_listen_port": 30730,
					  "movement_da_light_node_connection_protocol": "http",
					  "movement_da_light_node_connection_hostname": "movement-celestia-da-light-node",
					  "movement_da_light_node_connection_port": 30730,
					  "movement_da_light_node_http1": false,
					  "da_signers": {
						"private_key_hex": "<redacted-hex-string>",
						"public_keys_hex": [
						  "026df08227f565470ed01dbf56cde3fe5f66cc8ef793088cf68329bdd23a5d1f28"
						]
					  }
					},
					"celestia_force_new_chain": true,
					"memseq": {
					  "sequencer_chain_id": "d65d2d9df7f98e7df32c",
					  "sequencer_database_path": "/.movement/memseq/d65d2d9df7f98e7df32c/.memseq",
					  "memseq_build_time": 1000,
					  "memseq_max_block_size": 2048
					},
					"da_light_node_is_initial": false,
					"access_control": {
					  "ingress_account_whitelist": null
					},
					"digest_store": {
					  "digest_store_db_path": "/tmp/digest_store_db"
					}
				  }
				},
				"mcr": {
				  "eth_connection": {
					"eth_rpc_connection_protocol": "http",
					"eth_rpc_connection_hostname": "setup",
					"eth_rpc_connection_port": 8090,
					"eth_ws_connection_protocol": "ws",
					"eth_ws_connection_hostname": "setup",
					"eth_ws_connection_port": 8090,
					"eth_chain_id": 3073
				  },
				  "settle": {
					"should_settle": false,
					"signer_private_key": "<redacted-hex-string>",
					"mcr_contract_address": "0x5fc8d32690cc91d4c39d9d3abcbd16989f875707",
					"settlement_super_block_size": 1,
					"settlement_admin_mode": false
				  },
				  "transactions": {
					"gas_limit": 10000000000000000 as i64,
					"batch_timeout": 2000,
					"transaction_send_retries": 10
				  },
				  "maybe_run_local": true,
				  "deploy": {
					"mcr_deployment_working_directory": "protocol-units/settlement/mcr/contracts",
					"mcr_deployment_account_private_key": "<redacted-hex-string>"
				  },
				  "testing": null
				},
				"da_db": {
				  "da_db_path": "/.movement/movement-da-db"
				},
				"execution_extension": {
				  "block_retry_count": 10,
				  "block_retry_increment_microseconds": 5000
				},
				"syncing": {
				  "movement_sync": "leader::mainnet-l-sync-bucket-sync<=>{default_signer_address_whitelist,maptos,maptos-storage,movement-da-db}/**",
				  "application_id": [
					26,
					43,
					60,
					77,
					94,
					111,
					122,
					139,
					156,
					173,
					190,
					207,
					208,
					225,
					242,
					3,
					20,
					37,
					54,
					71,
					88,
					105,
					122,
					139,
					156,
					173,
					190,
					207,
					208,
					225,
					242,
					3
				  ],
				  "syncer_id": [
					157,
					176,
					168,
					220,
					173,
					5,
					115,
					141,
					114,
					148,
					25,
					118,
					79,
					232,
					97,
					122,
					208,
					240,
					178,
					127,
					137,
					16,
					141,
					102,
					130,
					237,
					209,
					161,
					166,
					182,
					69,
					205
				  ],
				  "root_dir": "/.movement"
				}
		});

		let config = ElsaToBiarritzRc1::migrate(json).unwrap();

		info!("{:#?}", config);

		assert_eq!(
			config.execution_config.maptos_config.chain.maptos_private_key_signer_identifier,
			SignerIdentifier::Local(Local {
				private_key_hex_bytes: "<redacted-hex-string>".to_string()
			})
		);

		let da_light_node_config = &config.celestia_da_light_node.celestia_da_light_node_config;
		let da_signers = match &da_light_node_config.network {
			movement_da_util::config::Network::Local => {
				&da_light_node_config.da_light_node.da_signers
			}
			_ => panic!("Expected Local"),
		};

		assert_eq!(
			da_signers.signer_identifier,
			SignerIdentifier::Local(Local {
				private_key_hex_bytes: "<redacted-hex-string>".to_string()
			})
		);

		assert_eq!(
			config.mcr.settle.signer_identifier,
			SignerIdentifier::Local(Local {
				private_key_hex_bytes: "<redacted-hex-string>".to_string()
			})
		);

		assert_eq!(
			config
				.mcr
				.deploy
				.ok_or_else(|| anyhow::anyhow!("deploy config not found"))?
				.signer_identifier,
			SignerIdentifier::Local(Local {
				private_key_hex_bytes: "<redacted-hex-string>".to_string()
			})
		);

		Ok(())
	}
}
