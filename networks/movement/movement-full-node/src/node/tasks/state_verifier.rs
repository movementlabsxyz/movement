use maptos_opt_executor::executor::ExecutionState;
use movement_da_sequencer_proto::MainNodeState;
use std::collections::BTreeMap;

const MAX_STATE_ENTRY: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeState {
	pub block_height: u64,
	pub ledger_timestamp: u64,
	pub ledger_version: u64,
}

pub struct StateVerifier {
	states: BTreeMap<u64, NodeState>,
}

impl StateVerifier {
	pub fn new() -> Self {
		StateVerifier { states: BTreeMap::new() }
	}

	pub fn validate(&self, local_state: &ExecutionState) -> bool {
		//if the height is not present, return true.
		self.states
			.get(&local_state.block_height.into())
			.map(|s| {
				let ledger_timestamp: u64 = local_state.ledger_timestamp.into();
				let ledger_version: u64 = local_state.ledger_version.into();
				s.ledger_timestamp == ledger_timestamp && s.ledger_version == ledger_version
			})
			.unwrap_or(true)
	}

	pub fn add_state(&mut self, main_node_state: MainNodeState) {
		if self.states.len() >= MAX_STATE_ENTRY {
			self.states.pop_first();
		}
		let new_state = NodeState {
			block_height: main_node_state.block_height,
			ledger_timestamp: main_node_state.ledger_timestamp,
			ledger_version: main_node_state.ledger_version,
		};
		// State can only be added once
		if !self.states.contains_key(&new_state.block_height) {
			self.states.insert(new_state.block_height, new_state);
		}
	}

	pub fn get_state(&self, block_height: u64) -> Option<&NodeState> {
		self.states.get(&block_height)
	}
}
#[cfg(test)]
mod test {

	use super::*;

	#[test]
	fn test_validate_state() {
		let mut state_verifier = StateVerifier::new();

		// Verify with no state stored. Validation true.
		let state1 = ExecutionState { block_height: 1, ledger_timestamp: 2, ledger_version: 3 };
		assert!(state_verifier.validate(&state1), "Empty state verifier validate a state");

		// Add the same state and validate it
		let new_state = MainNodeState { block_height: 1, ledger_timestamp: 2, ledger_version: 3 };
		state_verifier.add_state(new_state);
		assert!(state_verifier.validate(&state1), "Same state added doesn't valid.");

		// Add a different state for same height and validate it
		let state2 = ExecutionState { block_height: 1, ledger_timestamp: 3, ledger_version: 3 };
		assert!(!state_verifier.validate(&state2), "Diff ts state valid");
		let state3 = ExecutionState { block_height: 1, ledger_timestamp: 2, ledger_version: 4 };
		assert!(!state_verifier.validate(&state3), "Diff version state valid");

		// Add a different state with same key
		let new_state = MainNodeState { block_height: 1, ledger_timestamp: 3, ledger_version: 3 };
		state_verifier.add_state(new_state);
		assert!(state_verifier.validate(&state1), "State updated, old one doesn't validate");
		assert!(!state_verifier.validate(&state2), "State updated, new one is valid");

		// Fill the state, oldest height should be removed.
		for index in 0u64..MAX_STATE_ENTRY as u64 {
			let state = MainNodeState {
				block_height: index + 2,
				ledger_timestamp: index + 3,
				ledger_version: index + 4,
			};
			state_verifier.add_state(state);
		}
		// Previous diff state should validate on height 1 that has been removed
		assert!(state_verifier.validate(&state2), "Previous state2 not valid");
		assert!(state_verifier.validate(&state1), "Previous state3 not valid");
	}
}
