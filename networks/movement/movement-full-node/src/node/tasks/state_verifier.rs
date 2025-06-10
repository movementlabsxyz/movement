/// We’ve seen edge cases that cause node state divergence.
/// Without on-chain consensus, we need a “source of truth” node to decide which state is correct.
/// We call this the main node (to avoid confusion with the leader node, which has a different role); it broadcasts its state to all other nodes.
/// Each node verifies the state received from the DA-Sequencer against its own computed state after execution.
/// If the states diverge, the node stops processing and must restore its state to recover.
/// The structs in this module are used for state verification.
/// Both the locally computed state and the main node’s published state are verified.
/// Each comparison (local vs. main and main vs. local) uses a `StateVerifier`; the last `MAX_STATE_ENTRY` states (both received and computed) are stored.
/// When a new state is published or computed, verification occurs for that state’s height.
/// States at the same height must have the same ledger timestamp and version.
use maptos_opt_executor::executor::ExecutionState;
use movement_da_sequencer_proto::MainNodeState;
use std::collections::BTreeMap;

// We produce at most 2 blocks per second.
// With a `MAX_HISTORY_SIZE` of 120, a node that falls more than 60 s behind
// will not detect state divergence.
// We chose 120 (60 seconds) because any node lagging by more than 60 s
// likely has other issues and cannot be considered in sync.
const MAX_STATE_ENTRY: usize = 120;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeState {
	pub block_height: u64,
	pub ledger_timestamp: u64,
	pub ledger_version: u64,
}

impl From<&MainNodeState> for NodeState {
	fn from(main_node_state: &MainNodeState) -> Self {
		NodeState {
			block_height: main_node_state.block_height,
			ledger_timestamp: main_node_state.ledger_timestamp,
			ledger_version: main_node_state.ledger_version,
		}
	}
}

impl From<&ExecutionState> for NodeState {
	fn from(state: &ExecutionState) -> Self {
		NodeState {
			block_height: state.block_height,
			ledger_timestamp: state.ledger_timestamp,
			ledger_version: state.ledger_version,
		}
	}
}

pub struct StateVerifier {
	states: BTreeMap<u64, NodeState>,
}

impl StateVerifier {
	pub fn new() -> Self {
		StateVerifier { states: BTreeMap::new() }
	}

	pub fn validate(&self, local_state: &NodeState) -> bool {
		//if the height is not present, return true.
		self.states
			.get(&local_state.block_height.into())
			.map(|s| {
				s.ledger_timestamp == local_state.ledger_timestamp
					&& s.ledger_version == local_state.ledger_version
			})
			.unwrap_or(true)
	}

	pub fn add_state(&mut self, new_state: NodeState) {
		// State can only be added once
		if !self.states.contains_key(&new_state.block_height) {
			// If the number of stored states exceeds the maximum allowed entries,
			// remove the oldest entry (smallest key) to maintain a fixed-size cache.
			// This ensures that the most recent states are retained.
			if self.states.len() >= MAX_STATE_ENTRY {
				self.states.pop_first();
			}
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

	// Verify with no state stored. Validate true.
	#[test]
	fn test_state_validate_empty() {
		let state_verifier = StateVerifier::new();

		// Verify with no state stored. Validation true any state.
		let state1 = ExecutionState { block_height: 1, ledger_timestamp: 2, ledger_version: 3 };
		assert!(
			state_verifier.validate(&(&state1).into()),
			"Empty state verifier validate a state"
		);
	}

	// Add the same state and validate it
	#[test]
	fn test_state_validate_same_state() {
		let mut state_verifier = StateVerifier::new();
		// Add the same state and validate it
		let new_state = MainNodeState { block_height: 1, ledger_timestamp: 2, ledger_version: 3 };
		state_verifier.add_state((&new_state).into());
		let state1 = ExecutionState { block_height: 1, ledger_timestamp: 2, ledger_version: 3 };
		assert!(state_verifier.validate(&(&state1).into()), "Same state added doesn't valid.");
	}
	// Add a different state for same height and validate it
	#[test]
	fn test_state_validate_diff_state() {
		let mut state_verifier = StateVerifier::new();
		// Add initial state
		let new_state = MainNodeState { block_height: 1, ledger_timestamp: 2, ledger_version: 3 };
		state_verifier.add_state((&new_state).into());

		// Add a different state for same height and validate it
		let state2 = ExecutionState { block_height: 1, ledger_timestamp: 3, ledger_version: 3 };
		assert!(!state_verifier.validate(&(&state2).into()), "Diff ts and state valid");
		let state3 = ExecutionState { block_height: 1, ledger_timestamp: 2, ledger_version: 4 };
		assert!(!state_verifier.validate(&(&state3).into()), "Diff version and state valid");
	}

	// Add a different state with same height. First added still valid.
	#[test]
	fn test_state_validate_old_state_after_update() {
		let mut state_verifier = StateVerifier::new();
		let new_state = MainNodeState { block_height: 1, ledger_timestamp: 2, ledger_version: 3 };
		state_verifier.add_state((&new_state).into());

		// Add a different state with same height
		let new_state = MainNodeState { block_height: 1, ledger_timestamp: 3, ledger_version: 3 };
		state_verifier.add_state((&new_state).into());
		// Old state still validate.
		let state1 = ExecutionState { block_height: 1, ledger_timestamp: 2, ledger_version: 3 };
		assert!(
			state_verifier.validate(&(&state1).into()),
			"State updated, old one doesn't validate"
		);
		//New one invalid.
		let state2 = ExecutionState { block_height: 1, ledger_timestamp: 3, ledger_version: 3 };
		assert!(!state_verifier.validate(&(&state2).into()), "State updated, new one is valid");
	}

	// Fill the state, oldest height should be removed.
	#[test]
	fn test_state_validate_fill_state() {
		let mut state_verifier = StateVerifier::new();
		// Add the first state that should be removed with nely added state.
		let new_state = MainNodeState { block_height: 1, ledger_timestamp: 2, ledger_version: 3 };
		state_verifier.add_state((&new_state).into());

		//State change detection works before adding new state.
		let state2 = ExecutionState { block_height: 1, ledger_timestamp: 3, ledger_version: 3 };
		assert!(!state_verifier.validate(&(&state2).into()), "State updated, new one is valid");

		// Fill the state, oldest height should be removed.
		for index in 0u64..MAX_STATE_ENTRY as u64 {
			let state = MainNodeState {
				block_height: index + 2,
				ledger_timestamp: index + 3,
				ledger_version: index + 4,
			};
			state_verifier.add_state((&state).into());
		}
		// Any state should validate at height 1 now because the first state has been removed
		let state1 = ExecutionState { block_height: 1, ledger_timestamp: 3, ledger_version: 4 };
		assert!(state_verifier.validate(&(&state1).into()), "Previous state3 not valid");
	}
}
