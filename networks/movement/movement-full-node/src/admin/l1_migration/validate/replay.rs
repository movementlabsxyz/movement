use crate::admin::l1_migration::validate::compare::compare_transaction_outputs;
use crate::admin::l1_migration::validate::types::api::{AptosRestClient, MovementRestClient};
use crate::admin::l1_migration::validate::types::da::{get_da_block_height, DaSequencerClient};
use anyhow::Context;
use aptos_api_types::{AptosError, AptosErrorCode, Transaction};
use aptos_crypto::HashValue;
use aptos_rest_client::error::RestError;
use aptos_rest_client::Response;
use aptos_types::transaction::{SignedTransaction, TransactionPayload};
use clap::{Args, Parser};
use std::collections::HashMap;
use std::fmt::Display;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio_stream::StreamExt;
use tracing::{debug, error, info, warn};

const DEFAULT_MAX_SERVER_LAG_WAIT_DURATION: Duration = Duration::from_secs(60);
const LOG_PREFIX: &str = "@R:";
const SUBMISSION: &str = "S:";
const EXECUTION: &str = "E:";
const APTOS_FAILED: &str = "AF:";
const MOVEMENT_FAILED: &str = "MF:";
const BOTH_FAILED: &str = "BF:";
const BOTH_SUCCEEDED: &str = "BS:";

#[derive(Parser, Debug)]
#[clap(name = "replay", about = "Stream transactions from DA-sequencer blocks")]
pub struct DaReplayTransactions {
	#[clap(value_parser)]
	#[clap(long = "movement-api", help = "The url of the Movement full node endpoint")]
	pub movement_api_url: Option<String>,
	#[clap(long = "aptos-api", help = "The url of the Aptos validator node api endpoint")]
	pub aptos_api_url: String,
	#[clap(long = "da", help = "The url of the DA-Sequencer")]
	pub da_sequencer_url: String,
	#[command(flatten)]
	da_sequencer_db: DaBlockHeight,
	#[clap(long = "diff", help = "Show diff on transaction output mismatch")]
	pub show_diff: bool,
}

#[derive(Args, Debug)]
#[group(required = true, multiple = false)]
pub struct DaBlockHeight {
	#[arg(long = "da-db", help = "Path to the DA-Sequencer database")]
	pub path: Option<PathBuf>,
	#[arg(long = "da-height", help = "Synced DA-Sequencer block height")]
	pub height: Option<u64>,
}

impl DaReplayTransactions {
	pub async fn run(&self) -> anyhow::Result<()> {
		let block_height = match (self.da_sequencer_db.height, &self.da_sequencer_db.path) {
			(Some(height), _) => height,
			(_, Some(path)) => {
				let da_block_height = get_da_block_height(path)?;
				info!("Extracted DA-Sequencer block height: {}", da_block_height);
				da_block_height
			}
			_ => unreachable!(),
		};
		let (tx_batches, rx_batches) = mpsc::channel::<Vec<SignedTransaction>>(10);
		let mut tasks = JoinSet::new();
		let da_sequencer_client = DaSequencerClient::try_connect(&self.da_sequencer_url).await?;
		let aptos_rest_client = AptosRestClient::try_connect(&self.aptos_api_url).await?;

		// Spawn a task which compares transaction outputs from the Movement node and Aptos node
		let tx_validate_submission = if let Some(ref movement_api_url) = self.movement_api_url {
			let movement_rest_client = MovementRestClient::try_connect(movement_api_url).await?;
			let (tx_validate_execution, rx_validate_execution) =
				mpsc::unbounded_channel::<ValidateExecution>();
			let (tx_validate_submission, rx_validate_submission) =
				mpsc::unbounded_channel::<ValidateSubmission>();
			tasks.spawn(validate_transaction_execution(
				aptos_rest_client.clone(),
				movement_rest_client.clone(),
				rx_validate_execution,
				self.show_diff,
			));
			tasks.spawn(validate_transaction_submission(
				movement_rest_client,
				rx_validate_submission,
				tx_validate_execution,
			));
			Some(tx_validate_submission)
		} else {
			None
		};

		// Spawn a task which submits transaction batches to the validator node
		tasks.spawn(submit_transactions(aptos_rest_client, rx_batches, tx_validate_submission));
		// Spawn a task which fetches transaction batches ahead
		tasks.spawn(stream_transactions(da_sequencer_client, tx_batches, block_height));

		// If one of the tasks has finished then something went wrong
		tasks.join_next().await;
		tasks.shutdown().await;

		error!("Broken stream");
		Err(anyhow::anyhow!("Broken stream"))
	}
}

#[test]
fn verify_tool() {
	use clap::CommandFactory;
	DaReplayTransactions::command().debug_assert()
}

async fn stream_transactions(
	da_sequencer_client: DaSequencerClient,
	tx_batches: mpsc::Sender<Vec<SignedTransaction>>,
	block_height: u64,
) {
	if let Ok(stream) = da_sequencer_client.stream_transactions_from_height(block_height).await {
		let stream = stream.chunks_timeout(10, Duration::from_secs(1));

		futures::pin_mut!(stream);
		while let Some(txns) = stream.next().await {
			let txns = txns
				.into_iter()
				.collect::<Result<Vec<_>, _>>()
				.context("Failed to get the next batch of Aptos transactions");

			match txns {
				Ok(txns) => {
					if tx_batches.send(txns).await.is_err() {
						// channel is closed
						break;
					}
				}
				Err(err) => {
					error!("{err}");
					break;
				}
			}
		}
		warn!("Stream of transaction from the DA-Sequencer ended unexpectedly");
	} else {
		error!("Failed to stream transactions from DA-Sequencer blocks")
	}
}

async fn submit_transactions(
	aptos_rest_client: AptosRestClient,
	mut rx_batches: mpsc::Receiver<Vec<SignedTransaction>>,
	tx_validate_submission: Option<mpsc::UnboundedSender<ValidateSubmission>>,
) {
	while let Some(txns) = rx_batches.recv().await {
		match aptos_rest_client.submit_batch_bcs(&txns).await {
			Ok(result) => {
				debug!("Submitted {} Aptos transaction(s)", txns.len());

				let mut errors = result
					.into_inner()
					.transaction_failures
					.into_iter()
					.map(|item| (item.transaction_index, item.error))
					.collect::<HashMap<_, _>>();

				if let Some(ref tx_validate_submission) = tx_validate_submission {
					if txns
						.iter()
						.enumerate()
						.try_for_each(|(idx, txn)| {
							tx_validate_submission.send(ValidateSubmission {
								txn_info: TransactionInfo {
									hash: txn.committed_hash(),
									payload: payload_info(txn),
									expires: txn.expiration_timestamp_secs(),
								},
								error: errors.remove(&idx),
							})
						})
						.is_err()
					{
						// channel is closed
						break;
					}
				}
			}
			Err(e) => {
				error!("Failed to submit {} transaction(s): {}", txns.len(), e);
				break;
			}
		}
	}
	warn!("Stream of transaction batches ended unexpectedly");
}

async fn validate_transaction_execution(
	aptos_rest_client: AptosRestClient,
	movement_rest_client: MovementRestClient,
	mut rx_validate_execution: mpsc::UnboundedReceiver<ValidateExecution>,
	show_diff: bool,
) {
	use aptos_api_types::transaction::Transaction;

	while let Some(ValidateExecution { txn_info }) = rx_validate_execution.recv().await {
		let hash = txn_info.hash;
		let result = tokio::join!(
			movement_rest_client.wait_for_transaction_by_hash(
				hash,
				txn_info.expires,
				Some(DEFAULT_MAX_SERVER_LAG_WAIT_DURATION),
				None
			),
			aptos_rest_client.wait_for_transaction_by_hash(
				hash,
				txn_info.expires,
				Some(DEFAULT_MAX_SERVER_LAG_WAIT_DURATION),
				None,
			)
		);

		match result {
			(Ok(txn_movement), Ok(txn_aptos)) => {
				let Transaction::UserTransaction(txn_movement) = txn_movement.into_inner() else {
					unreachable!()
				};
				let Transaction::UserTransaction(txn_aptos) = txn_aptos.into_inner() else {
					unreachable!()
				};
				let result = compare_transaction_outputs(*txn_movement, *txn_aptos, show_diff);
				let (is_error, msg) = match (result.events_match, result.changes_match) {
					(true, true) => (false, "ok"),
					(true, false) => (true, "changes mismatch"),
					(false, true) => (true, "events mismatch"),
					(false, false) => (true, "events mismatch, changes mismatch"),
				};
				log_execution(is_error, BOTH_SUCCEEDED, hash, &txn_info.payload, msg);
			}
			(Ok(_), Err(error_aptos)) => {
				log_execution(true, APTOS_FAILED, hash, &txn_info.payload, error_aptos);
			}
			(Err(error_movement), Ok(_)) => {
				log_execution(true, MOVEMENT_FAILED, hash, &txn_info.payload, error_movement);
			}
			(Err(error_movement), Err(error_aptos)) => {
				let error_movement = format!("{}", error_movement);
				let error_aptos = format!("{}", error_aptos);

				if error_movement == error_aptos {
					log_execution(false, BOTH_FAILED, hash, &txn_info.payload, "same error");
				} else {
					log_execution(
						true,
						BOTH_FAILED,
						hash,
						&txn_info.payload,
						format!("(movement: {} // aptos: {})", error_movement, error_aptos),
					);
				}
			}
		}
	}
	warn!("Stream of transaction hashes ended unexpectedly");
}

fn log_execution(
	is_error: bool,
	result: &str,
	hash: HashValue,
	payload: &str,
	message: impl Display,
) {
	let msg = format!(
		"{}{}{}({}/{}): {}",
		LOG_PREFIX,
		EXECUTION,
		result,
		hash.to_hex_literal(),
		payload,
		message
	);
	if is_error {
		error!("{msg}");
	} else {
		info!("{msg}");
	}
}

async fn validate_transaction_submission(
	movement_rest_client: MovementRestClient,
	mut rx_validate_submission: mpsc::UnboundedReceiver<ValidateSubmission>,
	tx_validate_execution: mpsc::UnboundedSender<ValidateExecution>,
) {
	while let Some(ValidateSubmission { txn_info, error }) = rx_validate_submission.recv().await {
		let hash = txn_info.hash;
		let execute = error.is_none();
		let result = get_transaction_by_hash(&movement_rest_client, hash, 3).await;

		match (result, error) {
			(Ok(_), None) => {
				log_submission(false, BOTH_SUCCEEDED, hash, "ok");
			}
			(Ok(_), Some(error_aptos)) => {
				log_submission(true, APTOS_FAILED, hash, error_aptos);
			}
			(Err(error_movement), None) => {
				log_submission(true, MOVEMENT_FAILED, hash, error_movement);
			}
			(Err(mvmt_err), Some(error_aptos)) => {
				if let RestError::Http(_, _) = mvmt_err {
					log_submission(false, BOTH_FAILED, hash, "not executed on movement");
				} else {
					// Submission of the txn succeeded on Movement, but the execution failed.
					// However, we are logging only the Aptos submission error here
					// because we assume that if the submission had succeeded, the execution
					// would have failed on Aptos in the same way.
					log_submission(true, APTOS_FAILED, hash, error_aptos);
				}
			}
		};

		if execute {
			if tx_validate_execution.send(ValidateExecution { txn_info }).is_err() {
				// channel is closed
				break;
			}
		}
	}
}

async fn get_transaction_by_hash(
	movement_rest_client: &MovementRestClient,
	hash: HashValue,
	retries: u8,
) -> Result<Response<Transaction>, RestError> {
	for idx in (0..retries).into_iter().rev() {
		let result = movement_rest_client.get_transaction_by_hash(hash).await;
		match result {
			Ok(txn_movement) => {
				return Ok(txn_movement);
			}
			Err(err) if idx == 0 => {
				return Err(err);
			}
			Err(err) => match err {
				RestError::Api(ref api_err) => {
					if let AptosErrorCode::TransactionNotFound = api_err.error.error_code {
						tokio::time::sleep(Duration::from_secs(1)).await;
						continue;
					} else {
						return Err(err);
					}
				}
				_ => return Err(err),
			},
		}
	}
	unreachable!()
}

fn log_submission(is_error: bool, result: &str, hash: HashValue, message: impl Display) {
	let msg =
		format!("{}{}{}({}): {}", LOG_PREFIX, SUBMISSION, result, hash.to_hex_literal(), message);
	if is_error {
		error!("{msg}");
	} else {
		info!("{msg}");
	}
}

fn payload_info(txn: &SignedTransaction) -> String {
	match txn.payload() {
		TransactionPayload::Script(_) => "script".to_string(),
		TransactionPayload::ModuleBundle(_) => "depricated".to_string(),
		TransactionPayload::EntryFunction(ef) => {
			format!("entry:{}::{}", ef.module(), ef.function())
		}
		TransactionPayload::Multisig(sig) => format!("multisig:{}", sig.multisig_address.to_hex()),
	}
}

struct TransactionInfo {
	pub hash: HashValue,
	pub payload: String,
	pub expires: u64,
}

struct ValidateExecution {
	pub txn_info: TransactionInfo,
}

struct ValidateSubmission {
	pub txn_info: TransactionInfo,
	pub error: Option<AptosError>,
}
