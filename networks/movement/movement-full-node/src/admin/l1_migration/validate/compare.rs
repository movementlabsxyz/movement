use aptos_api_types::transaction::UserTransaction;
use aptos_api_types::{Event, WriteSetChange};
use tracing::error;

pub fn compare_transaction_outputs(
	movement_txn: UserTransaction,
	aptos_txn: UserTransaction,
	show_diff: bool,
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
		if show_diff {
			error!(
				"Transaction events mismatch ({})\n{}",
				txn_hash,
				display_diff(&movement_txn.events, &aptos_txn.events)?
			);
		} else {
			error!("Transaction events mismatch ({})", txn_hash,);
		}
		return Ok(false);
	}

	let movement_changes = movement_txn
		.info
		.changes
		.iter()
		.map(Into::<WriteSetChangeCompare>::into)
		.collect::<Vec<_>>();
	let aptos_changes = aptos_txn
		.info
		.changes
		.iter()
		.map(Into::<WriteSetChangeCompare>::into)
		.collect::<Vec<_>>();
	if movement_changes != aptos_changes {
		if show_diff {
			error!(
				"Transaction write-set mismatch ({})\n{}",
				txn_hash,
				display_diff(&movement_txn.info.changes, &aptos_txn.info.changes)?
			);
		} else {
			error!("Transaction write-set mismatch ({})", txn_hash,);
		}
		return Ok(false);
	}

	Ok(true)
}

fn display_diff<T>(movement_values: &[T], aptos_values: &[T]) -> anyhow::Result<String>
where
	T: serde::Serialize,
{
	let movement_json = serde_json::to_string_pretty(movement_values)?;
	let aptos_json = serde_json::to_string_pretty(aptos_values)?;
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

struct EventCompare<'a>(&'a Event);

impl<'a> PartialEq for EventCompare<'a> {
	fn eq(&self, other: &Self) -> bool {
		self.0.typ == self.0.typ && self.0.guid == other.0.guid
	}
}

impl<'a> From<&'a Event> for EventCompare<'a> {
	fn from(value: &'a Event) -> Self {
		Self(value)
	}
}

struct WriteSetChangeCompare<'a>(&'a WriteSetChange);

impl<'a> PartialEq for WriteSetChangeCompare<'a> {
	fn eq(&self, other: &Self) -> bool {
		match (self.0, other.0) {
			(WriteSetChange::DeleteModule(value1), WriteSetChange::DeleteModule(value2)) => {
				// Ignored fields: state_key_hash
				value1.address == value2.address && value1.module == value2.module
			}
			(WriteSetChange::DeleteResource(value1), WriteSetChange::DeleteResource(value2)) => {
				// Ignored fields: state_key_hash
				value1.address == value2.address && value1.resource == value2.resource
			}
			(WriteSetChange::DeleteTableItem(value1), WriteSetChange::DeleteTableItem(value2)) => {
				// Ignored fields: state_key_hash, data
				value1.key == value2.key && value1.handle == value2.handle
			}
			(WriteSetChange::WriteModule(value1), WriteSetChange::WriteModule(value2)) => {
				// Ignored fields: state_key_hash
				value1.address == value2.address && value1.data == value2.data
			}
			(WriteSetChange::WriteResource(value1), WriteSetChange::WriteResource(value2)) => {
				// Ignored fields: state_key_hash, data.data.0.values
				value1.address == value2.address
					&& value1.data.typ == value2.data.typ
					&& value1.data.data.0.keys().eq(value2.data.data.0.keys())
			}
			(WriteSetChange::WriteTableItem(value1), WriteSetChange::WriteTableItem(value2)) => {
				// Ignored fields: state_key_hash, value, data
				value1.key == value2.key && value1.handle == value2.handle
			}
			_ => false,
		}
	}
}

impl<'a> From<&'a WriteSetChange> for WriteSetChangeCompare<'a> {
	fn from(value: &'a WriteSetChange) -> Self {
		Self(value)
	}
}
