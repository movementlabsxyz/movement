use aptos_framework::{BuildOptions, BuiltPackage};
use aptos_sdk::types::account_address::AccountAddress;
use e2e_move_tests::{
	aptos_governance::{create_proposal_v2, get_remaining_voting_power, partial_vote, vote},
	assert_abort, assert_success, increase_lockup, setup_staking, MoveHarness,
};
use move_command_line_common::env::get_move_compiler_v2_from_env;
use move_model::metadata::CompilerVersion;
use once_cell::sync::Lazy;
use std::collections::BTreeMap;
use std::path::PathBuf;

pub static PROPOSAL_SCRIPTS: Lazy<BTreeMap<String, Vec<u8>>> = Lazy::new(build_scripts);

pub fn build_package(package_path: PathBuf, options: BuildOptions) -> anyhow::Result<BuiltPackage> {
	let mut options = options;
	if get_move_compiler_v2_from_env() {
		options.compiler_version = Some(CompilerVersion::V2_0);
	}
	BuiltPackage::build(package_path.to_owned(), options)
}

fn build_scripts() -> BTreeMap<String, Vec<u8>> {
	let package_folder = "vote.data";
	let package_names = vec!["enable_partial_governance_voting"];
	process_scripts(package_folder, package_names)
}

pub fn process_scripts(
	package_folder: &str,
	package_names: Vec<&str>,
) -> BTreeMap<String, Vec<u8>> {
	let mut scripts = BTreeMap::new();
	for package_name in package_names {
		let script = build_package(
			test_dir_path(format!("{}/{}", package_folder, package_name).as_str()),
			aptos_framework::BuildOptions::default(),
		)
		.expect("building packages with scripts must succeed")
		.extract_script_code()[0]
			.clone();
		scripts.insert(package_name.to_string(), script);
	}
	scripts
}

fn test_dir_path(s: &str) -> PathBuf {
	PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src").join("tests").join(s)
}

pub fn propose_post_l1_merge_with_full_governance() {
	let mut harness = MoveHarness::new();
	let validator_1 = harness.new_account_at(AccountAddress::from_hex_literal("0x123").unwrap());
	let validator_2 = harness.new_account_at(AccountAddress::from_hex_literal("0x234").unwrap());
	let validator_1_address = *validator_1.address();
	let validator_2_address = *validator_2.address();

	let stake_amount = 25_000_000;
	assert_success!(setup_staking(&mut harness, &validator_1, stake_amount));
	assert_success!(increase_lockup(&mut harness, &validator_1));
	assert_success!(setup_staking(&mut harness, &validator_2, stake_amount));
	assert_success!(increase_lockup(&mut harness, &validator_2));

	// === Construct the actual release upgrade ===
	use aptos_framework_set_feature_flags_release::SetFeatureFlags;
	use aptos_release_builder::components::feature_flags::{FeatureFlag, Features};
	use aptos_types::on_chain_config::FeatureFlag as AptosFeatureFlag;

	let mut aptos_feature_flags = AptosFeatureFlag::default_features();
	aptos_feature_flags.push(AptosFeatureFlag::DECOMMISSION_CORE_RESOURCES);

	let features = Features {
		enabled: aptos_feature_flags.into_iter().map(FeatureFlag::from).collect(),
		disabled: vec![],
	};

	let base_release = PostL1Merge::new(); // your feature logic here
	let with_features = SetFeatureFlags::new(base_release, features);
	let release_bundle = with_features.release_bundle().unwrap();
	let execution_hash = release_bundle.execution_hash.clone();

	// === Create a governance proposal with the release's execution hash ===
	let mut proposal_id = 0;
	assert_success!(create_proposal_v2(
		&mut harness,
		&validator_1,
		validator_1_address,
		execution_hash,
		vec![], // metadata_location
		vec![], // metadata_hash
		false   // not multi-step
	));

	// === Vote using full governance logic ===
	assert_success!(vote(&mut harness, &validator_2, validator_2_address, proposal_id, true));
	assert_success!(vote(&mut harness, &validator_1, validator_1_address, proposal_id, true));

	// assert_success!(resolve(&mut harness, proposal_id, validator_1_address));
}

/// Partial Vote Assumes core_resources signer and is used for testing
pub fn test_partial_vote() {
	// Genesis starts with one validator with index 0

	//TODO: Eventually remove MoveHarness and use the local running network
	let mut harness = MoveHarness::new();
	let validator_1 = harness.new_account_at(AccountAddress::from_hex_literal("0x123").unwrap());
	let validator_2 = harness.new_account_at(AccountAddress::from_hex_literal("0x234").unwrap());
	let validator_1_address = *validator_1.address();
	let validator_2_address = *validator_2.address();

	let stake_amount_1 = 25_000_000;
	assert_success!(setup_staking(&mut harness, &validator_1, stake_amount_1));
	assert_success!(increase_lockup(&mut harness, &validator_1));
	let stake_amount_2 = 25_000_000;
	assert_success!(setup_staking(&mut harness, &validator_2, stake_amount_2));
	assert_success!(increase_lockup(&mut harness, &validator_2));

	let mut proposal_id: u64 = 0;
	assert_success!(create_proposal_v2(
		&mut harness,
		&validator_2,
		validator_2_address,
		vec![1],
		vec![],
		vec![],
		true
	));
	// Voters can vote on a partial voting proposal but argument voting_power will be ignored.
	assert_success!(partial_vote(
		&mut harness,
		&validator_1,
		validator_1_address,
		proposal_id,
		100,
		true
	));
	// No remaining voting power.
	assert_eq!(get_remaining_voting_power(&mut harness, validator_1_address, proposal_id), 0);

	// Enable partial governance voting. In production it requires governance.
	let core_resources =
		harness.new_account_at(AccountAddress::from_hex_literal("0xA550C18").unwrap());
	let script_code = PROPOSAL_SCRIPTS
		.get("enable_partial_governance_voting")
		.expect("proposal script should be built");
	let txn = harness.create_script(&core_resources, script_code.clone(), vec![], vec![]);
	assert_success!(harness.run(txn));

	// If a voter has already voted on a proposal before partial voting is enabled, the voter cannot vote on the proposal again.
	assert_abort!(
		partial_vote(&mut harness, &validator_1, validator_1_address, proposal_id, 100, true),
		0x10005
	);

	assert_success!(create_proposal_v2(
		&mut harness,
		&validator_1,
		validator_1_address,
		vec![1],
		vec![],
		vec![],
		true
	));

	// Cannot vote on a non-exist proposal.
	let wrong_proposal_id: u64 = 2;
	assert_abort!(
		partial_vote(&mut harness, &validator_1, validator_1_address, wrong_proposal_id, 100, true),
		25863
	);

	proposal_id = 1;
	assert_eq!(
		get_remaining_voting_power(&mut harness, validator_1_address, proposal_id),
		stake_amount_1
	);
	assert_eq!(
		get_remaining_voting_power(&mut harness, validator_2_address, proposal_id),
		stake_amount_1
	);

	// A voter can vote on a proposal multiple times with both Yes/No.
	assert_success!(partial_vote(
		&mut harness,
		&validator_1,
		validator_1_address,
		proposal_id,
		100,
		true
	));
	assert_eq!(
		get_remaining_voting_power(&mut harness, validator_1_address, proposal_id),
		stake_amount_1 - 100
	);
	assert_success!(partial_vote(
		&mut harness,
		&validator_1,
		validator_1_address,
		proposal_id,
		1000,
		false
	));
	assert_eq!(
		get_remaining_voting_power(&mut harness, validator_1_address, proposal_id),
		stake_amount_1 - 1100
	);
	// A voter cannot use voting power more than it has.
	assert_success!(partial_vote(
		&mut harness,
		&validator_1,
		validator_1_address,
		proposal_id,
		stake_amount_1,
		true
	));
	assert_eq!(get_remaining_voting_power(&mut harness, validator_1_address, proposal_id), 0);
}

pub fn full_governance_vote() {
	// Set up harness and two validators
	let mut harness = MoveHarness::new();
	let validator_1 = harness.new_account_at(AccountAddress::from_hex_literal("0x123").unwrap());
	let validator_2 = harness.new_account_at(AccountAddress::from_hex_literal("0x234").unwrap());
	let validator_1_address = *validator_1.address();
	let validator_2_address = *validator_2.address();

	// Stake and lock up for both validators
	let stake_amount = 25_000_000;
	assert_success!(setup_staking(&mut harness, &validator_1, stake_amount));
	assert_success!(increase_lockup(&mut harness, &validator_1));
	assert_success!(setup_staking(&mut harness, &validator_2, stake_amount));
	assert_success!(increase_lockup(&mut harness, &validator_2));

	// Validator 1 creates a full governance proposal
	let mut proposal_id = 0;
	assert_success!(create_proposal_v2(
		&mut harness,
		&validator_1,
		validator_1_address,
		vec![1], // Dummy execution hash
		vec![],
		vec![],
		false // Not a multi-step proposal
	));

	// Validator 2 votes YES on the proposal using full voting power
	assert_success!(vote(&mut harness, &validator_2, validator_2_address, proposal_id, true));

	// Trying to vote again with the same validator should fail (double voting not allowed)
	assert_abort!(
		vote(&mut harness, &validator_2, validator_2_address, proposal_id, true),
		0x10004 // EALREADY_VOTED or equivalent error code
	);

	// Validator 1 votes NO on the same proposal
	assert_success!(vote(&mut harness, &validator_1, validator_1_address, proposal_id, false));

	// Check remaining voting power: both should now be zero
	assert_eq!(get_remaining_voting_power(&mut harness, validator_1_address, proposal_id), 0);
	assert_eq!(get_remaining_voting_power(&mut harness, validator_2_address, proposal_id), 0);
}
