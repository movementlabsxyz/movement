// Create the config that contains one Eth key that sign in local.
use movement_signer_config::KeyDefinition;
use movement_signer_config::KeyProvider;

fn main() {
	let key = KeyDefinition {
		name: "ETH_TEST_KEY1".to_string(),
		provider: KeyProvider::AWSKMS,
		id: String::new(),
	};
	movement_signer_setup::setup_sign_config(vec![key]).unwrap();
}
