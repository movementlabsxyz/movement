module test_token::TestToken {
    use aptos_framework::coin::{Coin, CoinInfo, MintCapability, register, mint};
    use aptos_framework::signer;

    /// The type of the TEST token
    struct TEST has key, store {}

    /// Initialize the TEST token
    public fun initialize_test_token(account: &signer) acquires CoinInfo {
        // Register the token in the account
        register<TEST>(account);

        // Acquire the mint capability for the TEST token
        let mint_cap = coin::borrow_mint_capability<TEST>(account);

        // Mint 1,000,000 TEST tokens to the account
        let minted_coins = mint<TEST>(1_000_000, &mint_cap);

        // Deposit the minted coins into the account
        coin::deposit_from_sender<TEST>(account, minted_coins);
    }

    /// Register the TEST token in an account
    public fun register_token(account: &signer) {
        register<TEST>(account);
    }
}

