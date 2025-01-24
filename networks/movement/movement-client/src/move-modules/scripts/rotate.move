script {
    use aptos_framework::account;

    /// Rotate the authentication key of an account, ensuring secure verification of both current and new keys.
    /// This prevents malicious attacks and ensures ownership of both current and new keys.

    // For full description of how rotation works see :
    // https://github.com/movementlabsxyz/aptos-core/blob/ac9de113a4afec6a26fe587bb92c982532f09d3a/aptos-move/framework/aptos-framework/sources/account.move#L298
    fun main(
        account: &signer,
        from_scheme: u8,
        from_public_key_bytes: vector<u8>,
        to_scheme: u8,
        to_public_key_bytes: vector<u8>,
        cap_rotate_key: vector<u8>,
        cap_update_table: vector<u8>,
    ) {
        // Call the `rotate_authentication_key` function from the `account` module
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
