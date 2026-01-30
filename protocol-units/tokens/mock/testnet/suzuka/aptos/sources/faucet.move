/// Basic faucet, allows to request fungible assets between intervals.
module mock::faucet {
    use std::signer;
    use aptos_framework::timestamp;
    use aptos_framework::fungible_asset::{Self, Metadata, FungibleStore};
    use aptos_framework::primary_fungible_store;
    use aptos_framework::object::{Self, Object, ExtendRef};
    use aptos_framework::simple_map::{Self, SimpleMap};

    /// Faucet already exists for this asset
    const ERR_FAUCET_EXISTS: u64 = 100;
    /// Faucet does not exist for this asset
    const ERR_FAUCET_NOT_EXISTS: u64 = 101;
    /// Account is restricted and must wait before next request
    const ERR_RESTRICTED: u64 = 102;

    struct FaucetRegistry has key {
        faucets: SimpleMap<address, FaucetData>
    }

    struct FaucetData has store {
        store: Object<FungibleStore>,
        extend_ref: ExtendRef,
        per_request: u64,
        period: u64
    }

    struct Restricted has key {
        restrictions: SimpleMap<address, u64> // metadata address -> timestamp
    }

    public fun create_faucet_internal(
        account: &signer,
        metadata: Object<Metadata>,
        amount_to_deposit: u64,
        per_request: u64,
        period: u64
    ) acquires FaucetRegistry {
        let account_addr = signer::address_of(account);
        let metadata_addr = object::object_address(&metadata);

        // Initialize registry if it doesn't exist
        if (!exists<FaucetRegistry>(account_addr)) {
            move_to(account, FaucetRegistry {
                faucets: simple_map::create()
            });
        };

        let registry = borrow_global_mut<FaucetRegistry>(account_addr);

        assert!(
            !simple_map::contains_key(&registry.faucets, &metadata_addr),
            ERR_FAUCET_EXISTS
        );

        // Create a named object to hold the faucet store
        let constructor_ref = &object::create_object(account_addr);
        let extend_ref = object::generate_extend_ref(constructor_ref);
        let store = fungible_asset::create_store(constructor_ref, metadata);

        // Transfer initial deposit to the faucet store
        let fa = primary_fungible_store::withdraw(account, metadata, amount_to_deposit);
        fungible_asset::deposit(store, fa);

        simple_map::add(&mut registry.faucets, metadata_addr, FaucetData {
            store,
            extend_ref,
            per_request,
            period
        });
    }

    public fun change_settings_internal(
        account: &signer,
        metadata: Object<Metadata>,
        per_request: u64,
        period: u64
    ) acquires FaucetRegistry {
        let account_addr = signer::address_of(account);
        let metadata_addr = object::object_address(&metadata);

        assert!(
            exists<FaucetRegistry>(account_addr),
            ERR_FAUCET_NOT_EXISTS
        );

        let registry = borrow_global_mut<FaucetRegistry>(account_addr);

        assert!(
            simple_map::contains_key(&registry.faucets, &metadata_addr),
            ERR_FAUCET_NOT_EXISTS
        );

        let faucet_data = simple_map::borrow_mut(&mut registry.faucets, &metadata_addr);
        faucet_data.per_request = per_request;
        faucet_data.period = period;
    }

    /// Deposit more assets to faucet.
    public fun deposit_internal(
        faucet_addr: address,
        metadata: Object<Metadata>,
        amount: u64
    ) acquires FaucetRegistry {
        let metadata_addr = object::object_address(&metadata);

        assert!(
            exists<FaucetRegistry>(faucet_addr),
            ERR_FAUCET_NOT_EXISTS
        );

        let registry = borrow_global<FaucetRegistry>(faucet_addr);

        assert!(
            simple_map::contains_key(&registry.faucets, &metadata_addr),
            ERR_FAUCET_NOT_EXISTS
        );

        let faucet_data = simple_map::borrow(&registry.faucets, &metadata_addr);

        // Create a signer for the faucet object to receive deposits
        let faucet_signer = &object::generate_signer_for_extending(&faucet_data.extend_ref);
        let fa = primary_fungible_store::withdraw(faucet_signer, metadata, amount);
        fungible_asset::deposit(faucet_data.store, fa);
    }

    public fun request_internal(
        account: &signer,
        faucet_addr: address,
        metadata: Object<Metadata>
    ) acquires FaucetRegistry, Restricted {
        let account_addr = signer::address_of(account);
        let metadata_addr = object::object_address(&metadata);

        assert!(
            exists<FaucetRegistry>(faucet_addr),
            ERR_FAUCET_NOT_EXISTS
        );

        let registry = borrow_global<FaucetRegistry>(faucet_addr);

        assert!(
            simple_map::contains_key(&registry.faucets, &metadata_addr),
            ERR_FAUCET_NOT_EXISTS
        );

        let faucet_data = simple_map::borrow(&registry.faucets, &metadata_addr);

        let now = timestamp::now_seconds();

        // Check and update restrictions
        if (!exists<Restricted>(account_addr)) {
            move_to(account, Restricted {
                restrictions: simple_map::create()
            });
        };

        let restricted = borrow_global_mut<Restricted>(account_addr);

        if (simple_map::contains_key(&restricted.restrictions, &metadata_addr)) {
            let last_request = simple_map::borrow(&restricted.restrictions, &metadata_addr);
            assert!(
                *last_request + faucet_data.period <= now,
                ERR_RESTRICTED
            );
            let last_request_mut = simple_map::borrow_mut(&mut restricted.restrictions, &metadata_addr);
            *last_request_mut = now;
        } else {
            simple_map::add(&mut restricted.restrictions, metadata_addr, now);
        };

        // Withdraw from faucet store and deposit to requester
        let faucet_signer = &object::generate_signer_for_extending(&faucet_data.extend_ref);
        let fa = fungible_asset::withdraw(faucet_signer, faucet_data.store, faucet_data.per_request);
        primary_fungible_store::deposit(account_addr, fa);
    }

    public entry fun create_faucet(
        account: &signer,
        metadata: Object<Metadata>,
        amount_to_deposit: u64,
        per_request: u64,
        period: u64
    ) acquires FaucetRegistry {
        create_faucet_internal(account, metadata, amount_to_deposit, per_request, period);
    }

    public entry fun change_settings(
        account: &signer,
        metadata: Object<Metadata>,
        per_request: u64,
        period: u64
    ) acquires FaucetRegistry {
        change_settings_internal(account, metadata, per_request, period);
    }

    public entry fun deposit(
        _account: &signer,
        metadata: Object<Metadata>,
        amount: u64
    ) acquires FaucetRegistry {
        deposit_internal(@mock, metadata, amount);
    }

    /// "Mints" (requests) fungible assets from the faucet.
    public entry fun mint(
        account: &signer,
        metadata: Object<Metadata>
    ) acquires FaucetRegistry, Restricted {
        request_internal(account, @mock, metadata);
    }

    public entry fun mintAll(
        account: &signer,
        metadata1: Object<Metadata>,
        metadata2: Object<Metadata>,
        metadata3: Object<Metadata>,
        metadata4: Object<Metadata>
    ) acquires FaucetRegistry, Restricted {
        request_internal(account, @mock, metadata1);
        request_internal(account, @mock, metadata2);
        request_internal(account, @mock, metadata3);
        request_internal(account, @mock, metadata4);
    }

    #[test_only]
    use aptos_framework::account;
    #[test_only]
    use std::string::utf8;
    #[test_only]
    use std::option;

    #[test_only]
    fun setup_test_token(admin: &signer): Object<Metadata> {
        // Create a test fungible asset
        let constructor_ref = &object::create_named_object(admin, b"TEST_TOKEN");
        primary_fungible_store::create_primary_store_enabled_fungible_asset(
            constructor_ref,
            option::none(),
            utf8(b"Test Token"),
            utf8(b"TEST"),
            6,
            utf8(b""),
            utf8(b""),
        );
        let mint_ref = fungible_asset::generate_mint_ref(constructor_ref);
        let metadata = object::object_from_constructor_ref<Metadata>(constructor_ref);

        // Mint tokens to admin for the faucet
        let fa = fungible_asset::mint(&mint_ref, 1_000_000_000_000);
        primary_fungible_store::deposit(signer::address_of(admin), fa);

        metadata
    }

    #[test(admin = @mock, user = @0x456, aptos_framework = @0x1)]
    fun test_mint(admin: &signer, user: &signer, aptos_framework: &signer) acquires FaucetRegistry, Restricted {
        // Setup
        timestamp::set_time_has_started_for_testing(aptos_framework);
        account::create_account_for_test(signer::address_of(admin));
        account::create_account_for_test(signer::address_of(user));

        // Create test token and faucet
        let test_metadata = setup_test_token(admin);
        let per_request = 60_000_000_000;
        let period = 3600;
        create_faucet(admin, test_metadata, 900_000_000_000, per_request, period);

        // User requests tokens from faucet
        mint(user, test_metadata);

        // Verify user received tokens
        let user_addr = signer::address_of(user);
        let balance = primary_fungible_store::balance(user_addr, test_metadata);
        assert!(balance == per_request, 0); // Should receive per_request amount
    }

    #[test_only]
    fun setup_four_test_tokens(admin: &signer): (Object<Metadata>, Object<Metadata>, Object<Metadata>, Object<Metadata>) {
        // Create USDC-like token
        let usdc_ref = &object::create_named_object(admin, b"TEST_USDC");
        primary_fungible_store::create_primary_store_enabled_fungible_asset(
            usdc_ref, option::none(), utf8(b"Test USDC"), utf8(b"USDC"), 6, utf8(b""), utf8(b""));
        let usdc_mint = fungible_asset::generate_mint_ref(usdc_ref);
        let usdc_metadata = object::object_from_constructor_ref<Metadata>(usdc_ref);
        primary_fungible_store::deposit(signer::address_of(admin), fungible_asset::mint(&usdc_mint, 1_000_000_000_000));

        // Create USDT-like token
        let usdt_ref = &object::create_named_object(admin, b"TEST_USDT");
        primary_fungible_store::create_primary_store_enabled_fungible_asset(
            usdt_ref, option::none(), utf8(b"Test USDT"), utf8(b"USDT"), 6, utf8(b""), utf8(b""));
        let usdt_mint = fungible_asset::generate_mint_ref(usdt_ref);
        let usdt_metadata = object::object_from_constructor_ref<Metadata>(usdt_ref);
        primary_fungible_store::deposit(signer::address_of(admin), fungible_asset::mint(&usdt_mint, 1_000_000_000_000));

        // Create WBTC-like token
        let wbtc_ref = &object::create_named_object(admin, b"TEST_WBTC");
        primary_fungible_store::create_primary_store_enabled_fungible_asset(
            wbtc_ref, option::none(), utf8(b"Test WBTC"), utf8(b"WBTC"), 8, utf8(b""), utf8(b""));
        let wbtc_mint = fungible_asset::generate_mint_ref(wbtc_ref);
        let wbtc_metadata = object::object_from_constructor_ref<Metadata>(wbtc_ref);
        primary_fungible_store::deposit(signer::address_of(admin), fungible_asset::mint(&wbtc_mint, 10_000_000_000));

        // Create WETH-like token
        let weth_ref = &object::create_named_object(admin, b"TEST_WETH");
        primary_fungible_store::create_primary_store_enabled_fungible_asset(
            weth_ref, option::none(), utf8(b"Test WETH"), utf8(b"WETH"), 8, utf8(b""), utf8(b""));
        let weth_mint = fungible_asset::generate_mint_ref(weth_ref);
        let weth_metadata = object::object_from_constructor_ref<Metadata>(weth_ref);
        primary_fungible_store::deposit(signer::address_of(admin), fungible_asset::mint(&weth_mint, 100_000_000_000));

        (usdc_metadata, usdt_metadata, wbtc_metadata, weth_metadata)
    }

    #[test(admin = @mock, user = @0x456, aptos_framework = @0x1)]
    fun test_mintAll(admin: &signer, user: &signer, aptos_framework: &signer) acquires FaucetRegistry, Restricted {
        // Setup
        timestamp::set_time_has_started_for_testing(aptos_framework);
        account::create_account_for_test(signer::address_of(admin));
        account::create_account_for_test(signer::address_of(user));

        // Create test tokens and faucets
        let (usdc_metadata, usdt_metadata, wbtc_metadata, weth_metadata) = setup_four_test_tokens(admin);

        create_faucet(admin, usdc_metadata, 900_000_000_000, 60_000_000_000, 3600);
        create_faucet(admin, usdt_metadata, 900_000_000_000, 60_000_000_000, 3600);
        create_faucet(admin, wbtc_metadata, 9_000_000_000, 100_000_000, 3600);
        create_faucet(admin, weth_metadata, 90_000_000_000, 2_000_000_000, 3600);

        // User requests all tokens from faucet
        mintAll(user, usdc_metadata, usdt_metadata, wbtc_metadata, weth_metadata);

        // Verify user received all tokens
        let user_addr = signer::address_of(user);
        let usdc_balance = primary_fungible_store::balance(user_addr, usdc_metadata);
        let usdt_balance = primary_fungible_store::balance(user_addr, usdt_metadata);
        let wbtc_balance = primary_fungible_store::balance(user_addr, wbtc_metadata);
        let weth_balance = primary_fungible_store::balance(user_addr, weth_metadata);

        assert!(usdc_balance == 60_000_000_000, 0);  // USDC per_request
        assert!(usdt_balance == 60_000_000_000, 1);  // USDT per_request
        assert!(wbtc_balance == 100_000_000, 2);     // WBTC per_request
        assert!(weth_balance == 2_000_000_000, 3);   // WETH per_request
    }

    #[test(admin = @mock, user = @0x456, aptos_framework = @0x1)]
    #[expected_failure(abort_code = ERR_RESTRICTED, location = Self)]
    fun test_mint_restriction_period(admin: &signer, user: &signer, aptos_framework: &signer) acquires FaucetRegistry, Restricted {
        // Setup
        timestamp::set_time_has_started_for_testing(aptos_framework);
        account::create_account_for_test(signer::address_of(admin));
        account::create_account_for_test(signer::address_of(user));

        // Create test token and faucet
        let test_metadata = setup_test_token(admin);
        create_faucet(admin, test_metadata, 900_000_000_000, 60_000_000_000, 3600);

        // First request should succeed
        mint(user, test_metadata);

        // Second request immediately should fail due to restriction period (3600 seconds)
        mint(user, test_metadata);
    }

    #[test(admin = @mock, user = @0x456, aptos_framework = @0x1)]
    fun test_mint_after_period(admin: &signer, user: &signer, aptos_framework: &signer) acquires FaucetRegistry, Restricted {
        // Setup
        timestamp::set_time_has_started_for_testing(aptos_framework);
        account::create_account_for_test(signer::address_of(admin));
        account::create_account_for_test(signer::address_of(user));

        // Create test token and faucet
        let test_metadata = setup_test_token(admin);
        let per_request = 60_000_000_000;
        create_faucet(admin, test_metadata, 900_000_000_000, per_request, 3600);

        // First request
        mint(user, test_metadata);

        let user_addr = signer::address_of(user);
        let balance_after_first = primary_fungible_store::balance(user_addr, test_metadata);
        assert!(balance_after_first == per_request, 0);

        // Fast forward time past the restriction period (3600 seconds)
        timestamp::fast_forward_seconds(3601);

        // Second request should now succeed
        mint(user, test_metadata);

        let balance_after_second = primary_fungible_store::balance(user_addr, test_metadata);
        assert!(balance_after_second == per_request * 2, 1); // Should have double the amount
    }
}
