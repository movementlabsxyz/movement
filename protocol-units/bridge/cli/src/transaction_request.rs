extern crate web3;

use web3::futures::Future;
use web3::types::{TransactionRequest, U256};

fn main() {
	let (_eloop, transport) = web3::transports::Http::new("http://localhost:8545").unwrap();

	let web3 = web3::Web3::new(transport);
	let accounts = web3.eth().accounts().wait().unwrap();

	let balance_before = web3.eth().balance(accounts[1], None).wait().unwrap();

	let tx = TransactionRequest {
		from: accounts[0],
		to: Some(accounts[1]),
		gas: None,
		gas_price: None,
		value: Some(U256::from(10000)),
		data: None,
		nonce: None,
		condition: None,
	};

	let tx_hash = web3.eth().send_transaction(tx).wait().unwrap();

	let balance_after = web3.eth().balance(accounts[1], None).wait().unwrap();

	println!("TX Hash: {:?}", tx_hash);
	println!("Balance before: {}", balance_before);
	println!("Balance after: {}", balance_after);
}
