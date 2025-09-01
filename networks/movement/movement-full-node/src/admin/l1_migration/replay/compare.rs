use anyhow::Context;
use aptos_api_types::transaction::UserTransaction;
use aptos_api_types::Event;
use tracing::error;

pub fn compare_transaction_outputs(
	movement_txn: UserTransaction,
	aptos_txn: UserTransaction,
) -> anyhow::Result<bool> {
	let txn_hash = movement_txn.info.hash.0.to_hex_literal();

	if movement_txn.info.hash != aptos_txn.info.hash {
		error!(
			"Transaction hash mismatch:\nMovement transaction hash:{}\nAptos transaction hash:{}",
			txn_hash,
			aptos_txn.info.hash.0.to_hex_literal()
		);
		return Ok(false);
	}

	let movement_events =
		movement_txn.events.iter().map(Into::<EventCompare>::into).collect::<Vec<_>>();
	let aptos_events = aptos_txn.events.iter().map(Into::<EventCompare>::into).collect::<Vec<_>>();
	if movement_events != aptos_events {
		let movement_values = movement_events
			.iter()
			.map(|event| event.to_json())
			.collect::<Result<Vec<_>, _>>()
			.context("Failed to serialize Movement events to json")?;
		let aptos_values = aptos_events
			.iter()
			.map(|event| event.to_json())
			.collect::<Result<Vec<_>, _>>()
			.context("Failed to serialize Aptes events to json")?;
		error!(
			"Transaction events mismatch ({})\n{}",
			txn_hash,
			display_diff(movement_values, aptos_values)?
		);
		return Ok(false);
	}

	if movement_txn.info.changes != aptos_txn.info.changes {
		let movement_values = movement_txn
			.info
			.changes
			.iter()
			.map(|change| serde_json::to_value(change))
			.collect::<Result<Vec<_>, _>>()
			.context("Failed to serialize Movement write-set changes to json")?;
		let aptos_values = aptos_txn
			.info
			.changes
			.iter()
			.map(|change| serde_json::to_value(change))
			.collect::<Result<Vec<_>, _>>()
			.context("Failed to serialize Aptos write-set changes to json")?;
		error!(
			"Transaction write-set mismatch ({})\n{}",
			txn_hash,
			display_diff(movement_values, aptos_values)?
		);
		return Ok(false);
	}

	Ok(true)
}

struct EventCompare<'a>(&'a Event);

impl<'a> EventCompare<'a> {
	pub fn to_json(&self) -> anyhow::Result<serde_json::Value> {
		let mut event = serde_json::Map::with_capacity(4);
		event.insert("sequence_number".to_owned(), serde_json::to_value(&self.0.sequence_number)?);
		event.insert("type".to_owned(), serde_json::to_value(&self.0.typ)?);
		event.insert("data".to_owned(), self.0.data.to_owned());
		Ok(serde_json::Value::Object(event))
	}
}

impl<'a> PartialEq for EventCompare<'a> {
	fn eq(&self, other: &Self) -> bool {
		self.0.typ == other.0.typ && self.0.data == other.0.data
	}
}

impl<'a> From<&'a Event> for EventCompare<'a> {
	fn from(value: &'a Event) -> Self {
		Self(value)
	}
}

fn display_diff(
	movement_values: Vec<serde_json::Value>,
	aptos_values: Vec<serde_json::Value>,
) -> anyhow::Result<String> {
	let movement_json = serde_json::to_string_pretty(&serde_json::Value::Array(movement_values))?;
	let aptos_json = serde_json::to_string_pretty(&serde_json::Value::Array(aptos_values))?;
	Ok(create_diff(&movement_json, &aptos_json)?)
}

fn create_diff(movement: &str, aptos: &str) -> anyhow::Result<String> {
	use console::Style;
	use similar::{ChangeTag, TextDiff};
	use std::fmt::Write;

	let mut out = String::with_capacity(movement.len() + aptos.len());
	let diff = TextDiff::from_lines(movement, aptos);
	let hunks = diff
		.grouped_ops(3)
		.into_iter()
		.filter(|ops| !ops.is_empty())
		.collect::<Vec<_>>();
	let last_hunk_idx = hunks.len() - 1;

	writeln!(out, "--- Movement full-node")?;
	writeln!(out, "+++ Aptos validator-node")?;
	for (idx, hunk) in hunks.iter().enumerate() {
		for op in hunk.iter() {
			for change in diff.iter_changes(op) {
				let (sign, style) = match change.tag() {
					ChangeTag::Delete => ("-", Style::new().red()),
					ChangeTag::Insert => ("+", Style::new().green()),
					ChangeTag::Equal => (" ", Style::new()),
				};
				write!(out, "{}{}", style.apply_to(sign).bold(), style.apply_to(change))?;
			}
		}
		if idx < last_hunk_idx {
			writeln!(out, "===")?;
		}
	}

	Ok(out)
}
