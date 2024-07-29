// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use aptos_indexer_grpc_utils::{config::IndexerGrpcCacheWorkerConfig, IndexerGrpcFileStoreConfig, types::RedisUrl};
use serde::{Deserialize, Serialize};
use url::Url;

const RUNTIME_WORKER_MULTIPLIER: usize = 2;

fn main() -> Result<()> {

    use tracing_subscriber::EnvFilter;

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let config = dot_movement.try_get_config_from_json::<suzuka_config::Config>()?;


    let fullnode_grpc_address = format!(
        "http://{}:{}",
        config.execution_config.maptos_config.client.maptos_indexer_grpc_connection_hostname,
        config.execution_config.maptos_config.client.maptos_indexer_grpc_connection_port
    );
    println!("Connecting to indexer gRPC server at: {}", fullnode_grpc_address.clone());

    let config = IndexerGrpcCacheWorkerConfig {
        fullnode_grpc_address: fullnode_grpc_address.clone(),
        file_store_config: IndexerGrpcFileStoreConfig {
           // variable provideded by gcs file store and local file store
           // https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/indexer-grpc/indexer-grpc-utils/src/config.rs#L45
        },
        redis_main_instance_address: RedisUrl::new("redis://localhost:6379")?,
        enable_cache_compression: true,
    };

    let num_cpus = num_cpus::get();
    let worker_threads = (num_cpus * RUNTIME_WORKER_MULTIPLIER).max(16);
    println!(
        "[Indexer cache worker] Starting cache worker tokio runtime: num_cpus={}, worker_threads={}",
        num_cpus, worker_threads
    );

    let mut builder = tokio::runtime::Builder::new_multi_thread();
    builder
        .disable_lifo_slot()
        .enable_all()
        .worker_threads(worker_threads)
        .build()
        .unwrap()
        .block_on(async {
            config.run().await
        })
}