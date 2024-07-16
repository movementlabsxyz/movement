module mock::tokens {
    use std::signer;
    use std::string::utf8;
    use mock::faucet;

    use aptos_framework::coin::{Self, MintCapability, FreezeCapability, BurnCapability};

    struct USDC {}
    struct USDT {}
    struct WBTC {}
    struct WETH {}

    struct Caps<phantom CoinType> has key {
        mint: MintCapability<CoinType>,
        freeze: FreezeCapability<CoinType>,
        burn: BurnCapability<CoinType>,
    }

    public entry fun initialize(admin: &signer) acquires Caps { 
        let (usdc_b, usdc_f, usdc_m) =
            coin::initialize<USDC>(admin,
                utf8(b"Circle"), utf8(b"USDC"), 6, true);
        let (usdt_b, usdt_f, usdt_m) =
            coin::initialize<USDT>(admin,
                utf8(b"Tether"), utf8(b"USDT"), 6, true);
        let (btc_b, btc_f, btc_m) =
            coin::initialize<BTC>(admin,
                utf8(b"Bitcoin"), utf8(b"BTC"), 8, true);

        move_to(admin, Caps<USDC> { mint: usdc_m, freeze: usdc_f, burn: usdc_b });
        move_to(admin, Caps<USDT> { mint: usdt_m, freeze: usdt_f, burn: usdt_b });
        move_to(admin, Caps<BTC> { mint: btc_m, freeze: btc_f, burn: btc_b });
        register_coins_all(admin);
        mint_coins(admin);
    }

    fun mint_coins(admin: &signer) acquires Caps {
        let admin_addr = signer::address_of(admin);
        let max_value = 18446744073709551615;
        let usdc_caps = borrow_global<Caps<USDC>>(admin_addr);
        let usdt_caps = borrow_global<Caps<USDT>>(admin_addr);
        let btc_caps = borrow_global<Caps<BTC>>(admin_addr);
        let usdc_coins = coin::mint<USDC>(max_value, &usdc_caps.mint);
        let usdt_coins = coin::mint<USDT>(max_value, &usdt_caps.mint);
        let btc_coins = coin::mint<BTC>(max_value, &btc_caps.mint);
        coin::deposit(admin_addr, usdc_coins);
        coin::deposit(admin_addr, usdt_coins);
        coin::deposit(admin_addr, btc_coins);
        faucet::create_faucet<USDC>(admin, max_value, 60_000_000_000_000, 3600);
        faucet::create_faucet<USDT>(admin, max_value, 60_000_000_000_000, 3600);
        faucet::create_faucet<WBTC>(admin, max_value, 100_000_000, 3600);
        faucet::create_faucet<WETH>(admin, max_value, 2000_000_000, 3600);
    }

    public entry fun register_coins_all(account: &signer) {
        let account_addr = signer::address_of(account);
        if (!coin::is_account_registered<USDC>(account_addr)) {
            coin::register<USDC>(account);
        };
        if (!coin::is_account_registered<USDT>(account_addr)) {
            coin::register<USDT>(account);
        };
        if (!coin::is_account_registered<WBTC>(account_addr)) {
            coin::register<WBTC>(account);
        };
        if (!coin::is_account_registered<WETH>(account_addr)) {
            coin::register<WETH>(account);
        };
    }
    }

    #[test (admin = @mock)]
    fun test_init(admin: &signer) acquires Caps {
        initialize(admin);
    }
}