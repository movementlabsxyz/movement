script {
    use aptos_framework::account;

    /// Perform both offering the rotation capability and rotating the authentication key of an account.
    /// This ensures that the recipient has the necessary capability before performing the key rotation.
    fun main(
        account: &signer,
        rotation_capability_sig_bytes: vector<u8>,
        from_scheme: u8,
        from_public_key_bytes: vector<u8>,
        to_scheme: u8,
        to_public_key_bytes: vector<u8>,
        cap_rotate_key: vector<u8>,
        cap_update_table: vector<u8>,
        account_scheme: u8,
        account_public_key_bytes: vector<u8>,
        recipient_address: address,
    ) {
        // Step 1: Offer rotation capability to the recipient
        account::offer_rotation_capability(
            account,
            rotation_capability_sig_bytes,
            account_scheme,
            account_public_key_bytes,
            recipient_address,
        );

        // Step 2: Rotate the authentication key
        account::rotate_authentication_key(
            account,
            from_scheme,
            from_public_key_bytes,
            to_scheme,
            to_public_key_bytes,
            cap_rotate_key,
            cap_update_table,
        );
    }
}
