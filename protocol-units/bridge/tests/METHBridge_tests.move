#[test_only]
module 0x1::METHBridgeTests {
    use 0x1::METHBridge;
    use 0x1::string;
    use std::signer;
    use aptos_framework::coin;
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::event;
    use aptos_framework::account::create_account_for_test;

    const BRIDGE_ACCOUNT: address = @0x1;
    const USER_ACCOUNT: address = @0x2;

    #[test(bridge = @0x1, user = @0x2)]
    fun test_deposit(bridge: signer, user: signer)
    acquires 0x1::METHBridge::BridgeAccount {
        // Initialize the module
        METHBridge::init_module(&bridge);

        let (burn_cap, freeze_cap, mint_cap) = coin::initialize<AptosCoin>(
            &bridge,
            string::utf8(b"MethCoin"),
            string::utf8(b"METH"),
            10,
            false,
        );
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_freeze_cap(freeze_cap);
        // Mint some coins to the user account
        let user_addr = signer::address_of(&user);
        create_account_for_test(user_addr);
        let coins = coin::mint<AptosCoin>(100, &mint_cap);
        coin::deposit(user_addr, coins);

        // Deposit coins from user to bridge
        let token_id = 1;
        let nonce = 1;
        let amount = 50;
        METHBridge::deposit(&bridge, user_addr, token_id, nonce, amount);

        // Verify user's balance
        assert!(coin::balance<AptosCoin>(user_addr) == 50, 1);

        // Verify event
        let bridge_account = borrow_global<METHBridge::BridgeAccount>(BRIDGE_ACCOUNT);
        let deposit_event = event::borrow_event<METHBridge::DepositEvent>(
            &bridge_account.deposit_events, 0
        );
        assert!(deposit_event.deposit.owner == user_addr, 1);
        assert!(deposit_event.deposit.token_id == token_id, 1);
        assert!(deposit_event.deposit.nonce == nonce, 1);
        assert!(deposit_event.deposit.amount == amount, 1);
    }

    #[test(bridge = @0x1, user = @0x2)]
    fun test_withdraw(bridge: signer, user: signer)
    acquires 0x1::METHBridge::BridgeAccount {
        // Initialize the module
        M2ETHBridge::init_module(&bridge);

        // Mint some coins to the user account
        let user_addr = signer::address_of(&user);
        create_account_for_test(user_addr);
        let coins = coin::mint<AptosCoin>(100, &user);
        coin::deposit(user_addr, coins);

        // Withdraw coins from user to bridge
        let token_id = 1;
        let amount = 50;
        METHBridge::withdraw(&user, token_id, amount);

        // Verify user's balance
        assert!(coin::balance<AptosCoin>(user_addr) == 50, 1);

        // Verify event
        let bridge_account = borrow_global<METHBridge::BridgeAccount>(BRIDGE_ACCOUNT);
        let withdrawal_event = event::borrow_event<METHBridge::PendingWithdrawalEvent>(
            &bridge_account.pending_withdrawal_events, 0
        );
        assert!(withdrawal_event.pending_withdrawal.request.owner == user_addr, 1);
        assert!(withdrawal_event.pending_withdrawal.request.token_id == token_id, 1);
        assert!(withdrawal_event.pending_withdrawal.request.amount == amount, 1);
        assert!(withdrawal_event.pending_withdrawal.nonce == 0, 1);
    }
}