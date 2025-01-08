module <address>::TestToken {
    use aptos_framework::coin::{self, Coin, register, mint};
    use aptos_framework::aptos_framework::{self};
    use aptos_framework::signer;
    
    /// The type of the TEST token
    struct TEST has key, store {}

    /// Initialize the TEST token
    public fun initialize_test_token(account: &signer) {
        // Register the token in the account
        register<TEST>(account);

        // Mint 1,000,000 TEST tokens to the account
        mint<TEST>(
            signer::address_of(account),
            1_000_000
        );
    }

    /// Register the TEST token in an account
    public fun register_token(account: &signer) {
        register<TEST>(account);
    }
}
