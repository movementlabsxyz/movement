// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use processor::IndexerGrpcProcessorConfig;
use processor::processors::ProcessorConfig;
use server_framework::RunnableConfig;
use ahash::AHashMap;

const RUNTIME_WORKER_MULTIPLIER: usize = 2;

fn main() -> Result<()> {

    let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let config = dot_movement.try_get_config_from_json::<suzuka_config::Config>()?;

    let config = IndexerGrpcProcessorConfig {
        processor_config: ProcessorConfig::DefaultProcessor,
        postgres_connection_string: config.execution_config.maptos_config.indexer_processor.postgres_connection_string.clone(),
        indexer_grpc_data_service_address: format!(
            "{}:{}", 
            config.execution_config.maptos_config.client.maptos_indexer_grpc_connection_hostname, 
            config.execution_config.maptos_config.client.maptos_indexer_grpc_connection_port
        ).parse()?,
        grpc_http2_config: Default::default(),
        auth_token: config.execution_config.maptos_config.indexer_processor.indexer_processor_auth_token.clone(),
        starting_version: None,
        ending_version: None,
        number_concurrent_processing_tasks: None,
        db_pool_size: None,
        gap_detection_batch_size: IndexerGrpcProcessorConfig::default_gap_detection_batch_size(),
        parquet_gap_detection_batch_size: IndexerGrpcProcessorConfig::default_gap_detection_batch_size(),
        pb_channel_txn_chunk_size: IndexerGrpcProcessorConfig::default_pb_channel_txn_chunk_size(),
        per_table_chunk_sizes: AHashMap::new(),
        enable_verbose_logging: None,
        grpc_response_item_timeout_in_secs: IndexerGrpcProcessorConfig::default_grpc_response_item_timeout_in_secs(),
        transaction_filter: Default::default(),
        deprecated_tables: Default::default(),
    };

    let num_cpus = num_cpus::get();
    let worker_threads = (num_cpus * RUNTIME_WORKER_MULTIPLIER).max(16);
    println!(
        "[Processor] Starting processor tokio runtime: num_cpus={}, worker_threads={}",
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
