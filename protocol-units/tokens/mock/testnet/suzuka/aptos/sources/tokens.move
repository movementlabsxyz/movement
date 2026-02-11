module mock::tokens {
    use std::signer;
    use std::string::utf8;
    use std::option;
    use mock::faucet;

    use aptos_framework::fungible_asset::{Self, MintRef, BurnRef, TransferRef, Metadata};
    use aptos_framework::object::{Self, Object};
    use aptos_framework::primary_fungible_store;

    struct Refs has key {
        usdc_mint_ref: MintRef,
        usdc_burn_ref: BurnRef,
        usdc_transfer_ref: TransferRef,
        usdt_mint_ref: MintRef,
        usdt_burn_ref: BurnRef,
        usdt_transfer_ref: TransferRef,
        wbtc_mint_ref: MintRef,
        wbtc_burn_ref: BurnRef,
        wbtc_transfer_ref: TransferRef,
        weth_mint_ref: MintRef,
        weth_burn_ref: BurnRef,
        weth_transfer_ref: TransferRef,
    }

    struct AssetRefs has key {
        usdc: Object<Metadata>,
        usdt: Object<Metadata>,
        wbtc: Object<Metadata>,
        weth: Object<Metadata>,
    }

    public entry fun initialize(admin: &signer) acquires Refs, AssetRefs {
        // Create USDC fungible asset
        let usdc_constructor_ref = &object::create_named_object(admin, b"USDC");
        primary_fungible_store::create_primary_store_enabled_fungible_asset(
            usdc_constructor_ref,
            option::none(),
            utf8(b"Circle"),
            utf8(b"USDC"),
            6,
            utf8(b""),
            utf8(b""),
        );
        let usdc_mint_ref = fungible_asset::generate_mint_ref(usdc_constructor_ref);
        let usdc_burn_ref = fungible_asset::generate_burn_ref(usdc_constructor_ref);
        let usdc_transfer_ref = fungible_asset::generate_transfer_ref(usdc_constructor_ref);
        let usdc_metadata = object::object_from_constructor_ref<Metadata>(usdc_constructor_ref);

        // Create USDT fungible asset
        let usdt_constructor_ref = &object::create_named_object(admin, b"USDT");
        primary_fungible_store::create_primary_store_enabled_fungible_asset(
            usdt_constructor_ref,
            option::none(),
            utf8(b"Tether"),
            utf8(b"USDT"),
            6,
            utf8(b""),
            utf8(b""),
        );
        let usdt_mint_ref = fungible_asset::generate_mint_ref(usdt_constructor_ref);
        let usdt_burn_ref = fungible_asset::generate_burn_ref(usdt_constructor_ref);
        let usdt_transfer_ref = fungible_asset::generate_transfer_ref(usdt_constructor_ref);
        let usdt_metadata = object::object_from_constructor_ref<Metadata>(usdt_constructor_ref);

        // Create WBTC fungible asset
        let wbtc_constructor_ref = &object::create_named_object(admin, b"WBTC");
        primary_fungible_store::create_primary_store_enabled_fungible_asset(
            wbtc_constructor_ref,
            option::none(),
            utf8(b"Bitcoin"),
            utf8(b"WBTC"),
            8,
            utf8(b""),
            utf8(b""),
        );
        let wbtc_mint_ref = fungible_asset::generate_mint_ref(wbtc_constructor_ref);
        let wbtc_burn_ref = fungible_asset::generate_burn_ref(wbtc_constructor_ref);
        let wbtc_transfer_ref = fungible_asset::generate_transfer_ref(wbtc_constructor_ref);
        let wbtc_metadata = object::object_from_constructor_ref<Metadata>(wbtc_constructor_ref);

        // Create WETH fungible asset
        let weth_constructor_ref = &object::create_named_object(admin, b"WETH");
        primary_fungible_store::create_primary_store_enabled_fungible_asset(
            weth_constructor_ref,
            option::none(),
            utf8(b"Ethereum"),
            utf8(b"WETH"),
            8,
            utf8(b""),
            utf8(b""),
        );
        let weth_mint_ref = fungible_asset::generate_mint_ref(weth_constructor_ref);
        let weth_burn_ref = fungible_asset::generate_burn_ref(weth_constructor_ref);
        let weth_transfer_ref = fungible_asset::generate_transfer_ref(weth_constructor_ref);
        let weth_metadata = object::object_from_constructor_ref<Metadata>(weth_constructor_ref);

        // Store all refs
        move_to(admin, Refs {
            usdc_mint_ref,
            usdc_burn_ref,
            usdc_transfer_ref,
            usdt_mint_ref,
            usdt_burn_ref,
            usdt_transfer_ref,
            wbtc_mint_ref,
            wbtc_burn_ref,
            wbtc_transfer_ref,
            weth_mint_ref,
            weth_burn_ref,
            weth_transfer_ref,
        });

        move_to(admin, AssetRefs {
            usdc: usdc_metadata,
            usdt: usdt_metadata,
            wbtc: wbtc_metadata,
            weth: weth_metadata,
        });

        mint_coins(admin);
    }

    fun mint_coins(admin: &signer) acquires Refs, AssetRefs {
        let admin_addr = signer::address_of(admin);
        let max_value = 18446744073709551615;
        let dexs = 10;

        let refs = borrow_global<Refs>(admin_addr);
        let asset_refs = borrow_global<AssetRefs>(admin_addr);

        // Mint USDC
        let usdc_fa = fungible_asset::mint(&refs.usdc_mint_ref, max_value);
        primary_fungible_store::deposit(admin_addr, usdc_fa);

        // Mint USDT
        let usdt_fa = fungible_asset::mint(&refs.usdt_mint_ref, max_value);
        primary_fungible_store::deposit(admin_addr, usdt_fa);

        // Mint WBTC
        let wbtc_fa = fungible_asset::mint(&refs.wbtc_mint_ref, max_value);
        primary_fungible_store::deposit(admin_addr, wbtc_fa);

        // Mint WETH
        let weth_fa = fungible_asset::mint(&refs.weth_mint_ref, max_value);
        primary_fungible_store::deposit(admin_addr, weth_fa);

        // Create faucets
        faucet::create_faucet(admin, asset_refs.usdc, max_value - (1_000_000_000_000 * dexs), 60_000_000_000, 3600);
        faucet::create_faucet(admin, asset_refs.usdt, max_value - (1_000_000_000_000 * dexs), 60_000_000_000, 3600);
        faucet::create_faucet(admin, asset_refs.wbtc, max_value - (1_700_000_000 * dexs), 100_000_000, 3600);
        faucet::create_faucet(admin, asset_refs.weth, max_value - (34_000_000_000 * dexs), 2000_000_000, 3600);
    }

    public entry fun register_coins_all(_account: &signer) {
        // With primary fungible stores, registration is automatic.
        // This function is kept for API compatibility but is now a no-op.
    }

    #[test (admin = @mock)]
    fun test_init(admin: &signer) acquires Refs, AssetRefs {
        initialize(admin);
    }
}