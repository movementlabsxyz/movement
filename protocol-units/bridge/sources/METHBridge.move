module 0x1::METHBridge {
    use std::signer;
    use std::vector;
    use aptos_framework::coin;
    use aptos_framework::event;
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::event::{EventHandle};
    use aptos_framework::account;

    const BRIDGE_ACCOUNT: address = @0x1; //Should change this later to actual bridge address

    struct BridgeAccount has key {
        deposit_events: EventHandle<DepositEvent>,
        pending_withdrawal_events: EventHandle<PendingWithdrawalEvent>,
        pending_withdrawals: vector<PendingWithdrawal>,
        nonce: u256,
    }

    struct Deposit has drop, store {
        owner: address,
        token_id: u128,
        nonce: u256,
        amount: u64,
    }

    struct DepositEvent has drop, store {
        deposit: Deposit,
    }

    struct PendingWithdrawalRequest has drop, store {
        owner: address,
        token_id: u128,
        amount: u64,
    }

    struct PendingWithdrawal has drop, store {
        request: PendingWithdrawalRequest,
        nonce: u256,
    }

    struct PendingWithdrawalEvent has drop, store {
        pending_withdrawal: PendingWithdrawal,
    }

    public entry fun deposit(
        trusted: &signer,
        deposit_owner: address,
        token_id: u128,
        nonce: u256,
        amount: u64
    ) acquires BridgeAccount {
        let trusted_address = signer::address_of(trusted);
        assert!(trusted_address == BRIDGE_ACCOUNT, 1); // Verify trusted signer

        let bridge_account = borrow_global_mut<BridgeAccount>(trusted_address);
        let coin = coin::withdraw<AptosCoin>(trusted, amount);
        coin::deposit(deposit_owner, coin);

        let deposit = Deposit {
            owner: deposit_owner,
            token_id,
            nonce,
            amount,
        };
        event::emit_event(
            &mut bridge_account.deposit_events,
            DepositEvent { deposit }
        );
    }

    public entry fun withdraw(
        owner: &signer,
        token_id: u128,
        amount: u64
    ) acquires BridgeAccount {
        let owner_address = signer::address_of(owner);
        let coin = coin::withdraw<AptosCoin>(owner, amount);
        let bridge_account = borrow_global_mut<BridgeAccount>(BRIDGE_ACCOUNT);
        coin::deposit(BRIDGE_ACCOUNT, coin);
        let nonce = bridge_account.nonce;
        bridge_account.nonce = nonce + 1;

        let request = PendingWithdrawalRequest {
            owner: owner_address,
            token_id,
            amount,
        };
        let pending_withdrawal = PendingWithdrawal {
            request,
            nonce,
        };
        event::emit_event(
            &mut bridge_account.pending_withdrawal_events,
            PendingWithdrawalEvent { pending_withdrawal }
        );
    }

    public entry fun close_withdrawal_request(
        trusted: &signer,
        owner: address,
        token_id: u128,
        nonce: u256
    ) acquires BridgeAccount {
        let trusted_address = signer::address_of(trusted);
        assert!(trusted_address == BRIDGE_ACCOUNT, 1); // Verify trusted signer

        let bridge_account = borrow_global_mut<BridgeAccount>(trusted_address);

        // Find the pending withdrawal with the matching owner, token_id, and nonce
        let pending_withdrawal_index = find_pending_withdrawal_index(bridge_account, owner, token_id, nonce);
        assert!(pending_withdrawal_index >= 0, 2); // Ensure the pending withdrawal exists

        // Remove the pending withdrawal from bridge_account
        let PendingWithdrawal { request, nonce: _ } = vector::remove(&mut bridge_account.pending_withdrawals, pending_withdrawal_index);

        // Transfer the coins back to the owner
        let PendingWithdrawalRequest { owner, token_id: _, amount } = request;
        let coin = coin::withdraw<AptosCoin>(trusted, amount);
        coin::deposit(owner, coin);

        // Emit an event to signal that the withdrawal request was closed
        event::emit_event(
            &mut bridge_account.pending_withdrawal_events,
            PendingWithdrawalEvent {
                pending_withdrawal: PendingWithdrawal {
                    request: PendingWithdrawalRequest {
                        owner,
                        token_id,
                        amount,
                    },
                    nonce,
                },
            },
        );
    }

    public entry fun claim_withdrawal_request(
        owner: &signer,
        owner_address: address,
        token_id: u128,
        nonce: u256
    ) acquires BridgeAccount {
        // Verify that the caller of this function owns the address they claim to
        assert!(owner_address == signer::address_of(owner), 1); // 1 indicates an authorization error

        let bridge_account = borrow_global_mut<BridgeAccount>(BRIDGE_ACCOUNT);
    
        // Find the index of the pending withdrawal that matches the provided parameters
        let pending_withdrawal_index = find_pending_withdrawal_index(
            bridge_account, 
            owner_address, 
            token_id, 
            nonce
        );
        assert!(pending_withdrawal_index != 0, 2); // 2 indicates that the pending withdrawal was not found
    
        // Remove the pending withdrawal from the list
        let PendingWithdrawal { request: _, nonce: _ } = vector::remove(
            &mut bridge_account.pending_withdrawals, 
            pending_withdrawal_index
        );
    
        // Emit an event to signal that the withdrawal claim was processed
        event::emit_event(
            &mut bridge_account.pending_withdrawal_events, 
            PendingWithdrawalEvent {
                pending_withdrawal: PendingWithdrawal {
                    request: PendingWithdrawalRequest {
                        owner: owner_address,
                        token_id,
                        amount: 0 // Since we're only closing the request, no actual transfer occurs
                    },
                    nonce
                }
            }
        );
    }

    fun find_pending_withdrawal_index(
        bridge_account: &BridgeAccount,
        owner: address,
        token_id: u128,
        nonce: u256,
    ): u64 {
        let pending_withdrawals = &bridge_account.pending_withdrawals;
        let i = 0;
        let len = vector::length(pending_withdrawals);
        while(i < len) {
            let pending_withdrawal = vector::borrow(pending_withdrawals, i);
            if (pending_withdrawal.request.owner == owner &&
                pending_withdrawal.request.token_id == token_id &&
                pending_withdrawal.nonce == nonce) {

                return i
            };
            i = i + 1;
        };

        return 0 // Return 0 if not found. Maybe also an error.
    }



    fun init_module(bridge: &signer) {
        let bridge_address = signer::address_of(bridge);
        assert!(bridge_address == BRIDGE_ACCOUNT, 1); // Verify bridge signer

        move_to(bridge, BridgeAccount {
            deposit_events: account::new_event_handle<DepositEvent>(bridge),
            pending_withdrawal_events: account::new_event_handle<PendingWithdrawalEvent>(bridge),
            pending_withdrawals: vector::empty(),
            nonce: 0,
        });
    }
    
    #[test_only]
    use aptos_framework::string;

    #[test(bridge = @0x1, user = @0x2)]
    fun test_deposit(bridge: signer, user: signer)
    acquires BridgeAccount {
        let (burn_cap, freeze_cap, mint_cap) = coin::initialize<AptosCoin>(
            &bridge,
            string::utf8(b"MethCoin"),
            string::utf8(b"METH"),
            10,
            false,
        );

        // We don't need these capabilities, we must explicitly destroy them 
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_freeze_cap(freeze_cap);

        // Mint some coins to the user account
        let user_addr = signer::address_of(&user);
        account::create_signer_for_test(user_addr);

         // Create the Account resource for the user and the bridge account
        account::create_account_for_test(user_addr);
        account::create_account_for_test(BRIDGE_ACCOUNT);

        // Register the user and bridge account to receive coins
        coin::register<AptosCoin>(&user);
        coin::register<AptosCoin>(&bridge);

        let coins = coin::mint<AptosCoin>(100, &mint_cap);
        coin::deposit(user_addr, coins);

        let bridge_addr = signer::address_of(&bridge);
        let coins = coin::mint<AptosCoin>(100, &mint_cap);
        coin::deposit(bridge_addr, coins);

        // Initialize the BridgeAccount resource
        init_module(&bridge);

        // Deposit coins from user to bridge
        let token_id = 1;
        let nonce = 1;
        let amount = 50;
        deposit(&bridge, user_addr, token_id, nonce, amount);

        // Verify user's balance
        // Original user_addr balance was 100, deposited 50, so new balance should be 150
        assert!(coin::balance<AptosCoin>(user_addr) == 150, 1);

        // Verify event
        let bridge_account = borrow_global<BridgeAccount>(BRIDGE_ACCOUNT);
        assert!(event::counter(&bridge_account.deposit_events) == 1, 1);

        coin::destroy_mint_cap(mint_cap);
    }

    #[test(bridge = @0x1, user = @0x2)]
    fun test_withdraw(bridge: signer, user: signer)
    acquires BridgeAccount {
        let (burn_cap, freeze_cap, mint_cap) = coin::initialize<AptosCoin>(
            &bridge,
            string::utf8(b"MethCoin"),
            string::utf8(b"METH"),
            10,
            false,
        );

        // We don't need these capabilities, we must explicitly destroy them 
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_freeze_cap(freeze_cap);

        // Create the Account resource for the user and the bridge account
        let user_addr = signer::address_of(&user);
        account::create_account_for_test(user_addr);
        account::create_account_for_test(BRIDGE_ACCOUNT);

        // Register the user and bridge account to receive coins
        coin::register<AptosCoin>(&user);
        coin::register<AptosCoin>(&bridge);

        // Mint some coins to the user account
        let coins = coin::mint<AptosCoin>(100, &mint_cap);
        coin::deposit(user_addr, coins);

        // Initialize the BridgeAccount resource
        init_module(&bridge);

        // Withdraw coins from user to bridge
        let token_id = 1;
        let amount = 50;
        0x1::METHBridge::withdraw(&user, token_id, amount);

        // Verify user's balance
        // Original user_addr balance was 100, withdrew 50, so new balance should be 50
        assert!(coin::balance<AptosCoin>(user_addr) == 50, 1);

        // Verify event
        let bridge_account = borrow_global<BridgeAccount>(BRIDGE_ACCOUNT);
        assert!(event::counter(&bridge_account.pending_withdrawal_events) == 1, 1);

        coin::destroy_mint_cap(mint_cap);
    }

    #[test(bridge = @0x1, user = @0x2)]
    fun test_close_withdrawal_request(bridge: signer, user: signer)
    acquires BridgeAccount {
        let (burn_cap, freeze_cap, mint_cap) = coin::initialize<AptosCoin>(
            &bridge,
            string::utf8(b"MethCoin"),
            string::utf8(b"METH"),
            10,
            false,
        );

        // Destroy unused capabilities
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_freeze_cap(freeze_cap);

        // Create the Account resource for the user and the bridge account
        let user_addr = signer::address_of(&user);
        account::create_account_for_test(user_addr);
        account::create_account_for_test(BRIDGE_ACCOUNT);

        // Register the user and bridge account to receive coins
        coin::register<AptosCoin>(&user);
        coin::register<AptosCoin>(&bridge);

        // Mint some coins to the user account
        let coins = coin::mint<AptosCoin>(100, &mint_cap);
        coin::deposit(user_addr, coins);

        // Initialize the BridgeAccount resource
        init_module(&bridge);

        // Perform a deposit and then a withdrawal to generate a pending withdrawal
        let token_id = 1;
        let nonce = 1;
        let deposit_amount = 50;
        let withdraw_amount = 30;
        deposit(&bridge, user_addr, token_id, nonce, deposit_amount);
        withdraw(&user, token_id, withdraw_amount);

        // Explicitly drop bridge_account borrow before calling close_withdrawal_request
        {
            let bridge_account = borrow_global<BridgeAccount>(BRIDGE_ACCOUNT);

            // Verify initial state
            assert!(coin::balance<AptosCoin>(user_addr) == 70, 1); // 100 - 30 = 70
            assert!(event::counter(&bridge_account.pending_withdrawal_events) == 1, 2);
            // Explicitly end the scope of `bridge_account` to release borrow
        };

        // Close the withdrawal request
        close_withdrawal_request(&bridge, user_addr, token_id, nonce);

        // Verify the pending withdrawals are cleared and coins are refunded
        let bridge_account = borrow_global<BridgeAccount>(BRIDGE_ACCOUNT); // Re-borrow bridge_account
        assert!(vector::is_empty<PendingWithdrawal>(&bridge_account.pending_withdrawals), 3);
        assert!(coin::balance<AptosCoin>(user_addr) == 100, 4); // 70 + 30 = 100
        assert!(event::counter<PendingWithdrawalEvent>(&bridge_account.pending_withdrawal_events) == 2, 5); // One for withdrawal and one for closure

        // Clean up
        coin::destroy_mint_cap(mint_cap);
    }
}