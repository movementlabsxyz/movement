script {
    use aptos_framework::account;

    fun rotate_authentication_key(account: &signer, new_public_key: vector<u8>) {
        // Derive the new authentication key from the public key
        let new_auth_key = account::authentication_key_from_public_key(&new_public_key);
        
        // Update the authentication key of the account
        account::rotate_authentication_key(account, new_auth_key);
    }
}
