// Create the config that contains one Eth key that sign using AWS KMS.
use godfig::env_default;
use movement_signer_config::KeyDefinition;
use movement_signer_config::KeyProvider;

fn main() {
	env_default!(get_aws_key_id, "AWS_KEY_ID", String);
	let awskms_key_id = get_aws_key_id().expect("AWS_KEY_ID not defined in env.");
	let key = KeyDefinition {
		name: "ETH_TEST_KEY1".to_string(),
		provider: KeyProvider::LOCALETH,
		id: awskms_key_id,
	};
	movement_signer_setup::setup_sign_config(vec![key]).unwrap();
}
