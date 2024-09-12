pub async fn create_fake_signed_transaction(
	chain_id: us,
	from_account: &LocalAccount,
	to_account: AccountAddress,
	amount: u64,
) -> SignedTransaction {
	let coin_type = "0x1::aptos_coin::AptosCoin";
	let timeout_secs = 600; // 10 minutes
	let max_gas_amount = 5_000;
	let gas_unit_price = 100;

	let transaction_builder =
		TransactionBuilder::new(TransactionPayload::EntryFunction(EntryFunction::new(
			ModuleId::new(AccountAddress::ONE, Identifier::new("coin").unwrap()),
			Identifier::new("transfer"),
			vec![TypeTag::from_str(coin_type).unwrap()],
			vec![
				to_bytes(&from_account).unwrap(),
				to_bytes(&to_account).unwrap(),
				to_bytes(&amount).unwrap(),
			],
		)));

	let expiration_time =
		SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() + timeout_secs;

	let raw_txn = transaction_builder
		.sender(from_account.address())
		.sequence_number(from_account.sequence_number())
		.max_gas_amount(max_gas_amount)
		.gas_unit_price(gas_unit_price)
		.expiration_time(expiration_time)
		.chain_id(ChainId::new(chain_id))
		.build();

	let signed_txn = from_account.sign_transaction(raw_txn);
}
