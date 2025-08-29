use aptos_api_types::TransactionOnChainData;
use aptos_types::contract_event::{ContractEvent, ContractEventV1, ContractEventV2};
use tracing::error;

pub fn compare_transaction_outputs(
	movement_txn: TransactionOnChainData,
	aptos_txn: TransactionOnChainData,
) -> bool {
	let txn_hash = movement_txn.info.transaction_hash().to_hex_literal();

	if movement_txn.info.transaction_hash() != aptos_txn.info.transaction_hash() {
		error!(
			"Transaction hash mismatch:\nMovement transaction hash:{}\nAptos transaction hash:{}",
			txn_hash,
			aptos_txn.info.transaction_hash().to_hex_literal()
		);
		return false;
	}

	let movement_events = movement_txn.events.iter().map(Into::<Event>::into).collect::<Vec<_>>();
	let aptos_events = movement_txn.events.iter().map(Into::<Event>::into).collect::<Vec<_>>();
	if movement_events != aptos_events {
		error!(
			"Transaction events mismatch ({})\nMovement events:\n{}\nAptos events:\n{}",
			txn_hash,
			display_events(&movement_txn.events),
			display_events(&aptos_txn.events)
		);
		return false;
	}

	if movement_txn.changes != aptos_txn.changes {
		error!("Transaction write-set mismatch ({})", txn_hash);
		return false;
	}

	true
}

fn display_events(events: &[ContractEvent]) -> String {
	format!("[\n  {}\n]", events.iter().map(|e| e.to_string()).collect::<Vec<_>>().join(",\n  "))
}

#[derive(PartialEq)]
enum Event<'a> {
	V1(EventV1<'a>),
	V2(EventV2<'a>),
}

impl<'a> From<&'a ContractEvent> for Event<'a> {
	fn from(value: &'a ContractEvent) -> Self {
		match value {
			ContractEvent::V1(e) => Event::V1(EventV1::from(e)),
			ContractEvent::V2(e) => Event::V2(EventV2::from(e)),
		}
	}
}

struct EventV1<'a>(&'a ContractEventV1);

impl<'a> From<&'a ContractEventV1> for EventV1<'a> {
	fn from(value: &'a ContractEventV1) -> Self {
		EventV1(value)
	}
}

impl<'a> PartialEq for EventV1<'a> {
	fn eq(&self, other: &Self) -> bool {
		self.0.key() == other.0.key()
			&& self.0.type_tag() == other.0.type_tag()
			&& self.0.event_data() == other.0.event_data()
	}
}

#[derive(PartialEq)]
struct EventV2<'a>(&'a ContractEventV2);

impl<'a> From<&'a ContractEventV2> for EventV2<'a> {
	fn from(value: &'a ContractEventV2) -> Self {
		EventV2(value)
	}
}
