script {
    use aptos_framework::account;

    fun main(
      account: &signer,                 // The signer of the transaction
      from_scheme: u8,                 // Key scheme of the current key
      from_public_key_bytes: vector<u8>, // Current public key in byte vector format
      to_scheme: u8,                   // Key scheme of the new key
      to_public_key_bytes: vector<u8>,   // New public key in byte vector format
      cap_rotate_key: vector<u8>,      // Signature or capability to rotate the key
      cap_update_table: vector<u8>,    // Optional update table for capabilities
    ) {
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
