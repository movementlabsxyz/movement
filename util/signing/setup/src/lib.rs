use movement_signer_config::KeyDefinition;

/// Save the provided Key definition in the signing KeyManager Config.
/// Call be each signing user during setup.
pub fn setup_sign_config(key_list: Vec<KeyDefinition>) -> Result<(), anyhow::Error> {
	todo!();
}
