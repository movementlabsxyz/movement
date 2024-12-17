//use bridge_indexer_db::client::Client as IndexerClient;
use bridge_util::{
	actions::{ActionExecError, TransferAction, TransferActionType},
	chains::bridge_contracts::BridgeContractEvent,
	events::{InvalidEventError, TransferEvent},
	states::TransferState,
	types::BridgeTransferId,
};
use std::collections::HashMap;

pub struct Runtime {
	swap_state_map: HashMap<BridgeTransferId, TransferState>,
}

impl Runtime {
	pub fn new() -> Self {
		Runtime { swap_state_map: HashMap::new() } //indexer_db_client
	}

	pub fn iter_state(&self) -> impl Iterator<Item = &TransferState> {
		self.swap_state_map.values()
	}

	pub fn remove_transfer(&mut self, transfer_id: BridgeTransferId) {
		self.swap_state_map.remove(&transfer_id);
	}

	pub fn process_event<A>(
		&mut self,
		event: TransferEvent<A>,
	) -> Result<TransferAction, InvalidEventError>
	where
		A: Into<Vec<u8>> + std::clone::Clone + std::fmt::Debug,
	{
		self.validate_state(&event)?;
		let event_transfer_id = event.contract_event.bridge_transfer_id();
		let state_opt = self.swap_state_map.remove(&event_transfer_id);
		let (state, action) = match event.contract_event {
			BridgeContractEvent::Initiated(detail) => {
				TransferState::transition_from_initiated(event_transfer_id, detail)
			}
			BridgeContractEvent::Completed(_detail) => {
				let state = state_opt.ok_or(InvalidEventError::BadEvent(
					"Receive an invalid even after validation.".to_string(),
				))?;
				let (new_state, action_kind) = state.transition_from_completed(event_transfer_id);
				let action =
					TransferAction { transfer_id: new_state.transfer_id, kind: action_kind };
				(new_state, action)
			}
		};
		self.swap_state_map.insert(state.transfer_id, state);
		Ok(action)
	}

	fn validate_state<A: std::fmt::Debug>(
		&mut self,
		event: &TransferEvent<A>,
	) -> Result<(), InvalidEventError> {
		let event_transfer_id = event.contract_event.bridge_transfer_id();
		let swap_state_opt = self.swap_state_map.get(&event_transfer_id);

		// Log the current state if it exists in the swap state map
		if let Some(state) = swap_state_opt {
			tracing::info!(
				"Found existing state for transfer ID {}: {}",
				event_transfer_id,
				state.state
			);
		} else {
			tracing::info!("No existing state found for transfer ID {}", event_transfer_id);
		}
		//validate the associated swap_state.
		swap_state_opt
			.as_ref()
			//if the state is present validate the event is compatible
			.map(|state| state.validate_event(&event))
			//if not validate the event is BridgeContractEvent::Initiated
			.or_else(|| {
				Some(
					(swap_state_opt.is_none() && event.contract_event.is_initiated_event())
						.then_some(())
						.ok_or(InvalidEventError::StateNotFound),
				)
			})
			.transpose()?;
		Ok(())
	}

	pub fn process_action_exec_error(
		&mut self,
		action_err: ActionExecError,
	) -> Option<TransferAction> {
		// Manage Tx execution error
		let (action, err) = action_err.inner();
		tracing::warn!("Client execution error for action:{action} err:{err}");
		// retry 5 time an action in error then abort.
		match self.swap_state_map.get_mut(&action.transfer_id) {
			Some(state) => {
				state.retry_on_error += 1;
				if state.retry_on_error < 6 {
					// Depending on the action cancel transfer
					match action.kind {
						TransferActionType::CompleteBridgeTransfer { .. }
						| TransferActionType::AbortedReplay { .. } => {
							//Complete failed. retry
							let transfer_id = state.transfer_id;
							let action_kind = state.transition_from_aborted(transfer_id);
							let action = TransferAction {
								transfer_id: state.transfer_id,
								kind: action_kind,
							};
							Some(action)
						}
						//Could never errors.
						TransferActionType::CompletedRemoveState => None,
						TransferActionType::NoAction => None,
					}
				} else {
					tracing::warn!("Relayer transfer failed: {} because send complete Tx failed more than 5 times.", state.transfer_id);
					None
				}
			}
			None => {
				tracing::warn!(
					"Receive an error for action but no state found for id:{}",
					action.transfer_id
				);
				None
			}
		}
	}
}
