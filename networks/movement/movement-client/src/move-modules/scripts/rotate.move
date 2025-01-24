script {
    use aptos_framework::account;

    fun rotate_authentication_key(account: &signer, new_auth_key: vector<u8>) {
        // Ensure the new authentication key is valid (32 bytes)
        assert!(
            vector::length(&new_auth_key) == 32,
            0 // Abort code for invalid key length
        );

        account::rotate_authentication_key_call(account, new_auth_key);
    }
}

