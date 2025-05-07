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
		self.states.insert(new_state.block_height, new_state);
	}

	pub fn get_state(&self, block_height: u64) -> Option<&NodeState> {
		self.states.get(&block_height)
	}
}
