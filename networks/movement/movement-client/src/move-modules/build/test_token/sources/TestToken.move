module test_token::TestToken {
    use aptos_framework::coin::{Coin, register};
    use aptos_framework::signer;

    /// The type of the TEST token
    struct TEST has key, store {}

    /// Mint capability for the TEST token
    struct MintCapability has store, key {}

    /// Initialize the TEST token
    public fun initialize_test_token(account: &signer) acquires MintCapability {
        // Register the token in the account
        register<TEST>(account);

        // Create and store the mint capability in the account
        move_to(account, MintCapability {});

        // Mint 1,000,000 TEST tokens to the account
        mint_to(account, 1_000_000);
    }

    /// Mint tokens to the account
    public fun mint_to(account: &signer, amount: u64) acquires MintCapability {
        let cap = borrow_global<MintCapability>(signer::address_of(account));
        // Logic to mint and deposit coins goes here
        // Replace this comment with minting logic for your token
    }

    /// Register the TEST token in an account
    public fun register_token(account: &signer) {
        register<TEST>(account);
    }
}
