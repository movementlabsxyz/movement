module test_token::GGPTestToken {
    use aptos_framework::coin::{Coin, BurnCapability, FreezeCapability, MintCapability, register, mint};
    use aptos_framework::signer;
    use aptos_framework::coin;
    use std::string;

    /// The type of the TEST token
    struct TEST has key, store {}

    /// Struct to hold all capabilities for the TEST token
    struct TokenCapabilities has key {
        burn: BurnCapability<TEST>,
        freeze: FreezeCapability<TEST>,
        mint: MintCapability<TEST>,
    }

    /// Initialize the TEST token
    public entry fun initialize_test_token(account: &signer) acquires TokenCapabilities {
        // Initialize the TEST token and get the capabilities
        let (burn_cap, freeze_cap, mint_cap) = coin::initialize<TEST>(
            account,
            string::utf8(b"GGP Test Token"), // Name of the token
            string::utf8(b"GT"),            // Symbol of the token
            6,                              // Number of decimals
            true                            // Monitor supply
        );

        // Register the TEST token for the account
        register<TEST>(account);

        // Create and store the capabilities in the account
        move_to(account, TokenCapabilities {
            burn: burn_cap,
            freeze: freeze_cap,
            mint: mint_cap,
        });

        // Mint 1,000,000 TEST tokens to the account
        mint_to(account, 1_000_000);
    }

    /// Mint tokens to the account
    public fun mint_to(account: &signer, amount: u64) acquires TokenCapabilities {
        // Borrow the `TokenCapabilities` from the account
        let token_caps = borrow_global<TokenCapabilities>(signer::address_of(account));

        // Use the `MintCapability` from `TokenCapabilities`
        let minted_coins = coin::mint<TEST>(amount, &token_caps.mint);

        // Deposit the minted coins into the account
        coin::deposit<TEST>(signer::address_of(account), minted_coins);
    }

    /// Register the TEST token in an account
    public fun register_token(account: &signer) {
        register<TEST>(account);
    }
}

