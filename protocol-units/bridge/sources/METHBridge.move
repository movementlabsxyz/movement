module 0x1::M2ETHBridge {
    use std::signer;
    use aptos_framework::coin;
    use aptos_framework::event;
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::event::{EventHandle};
    use aptos_framework::account;

    const BRIDGE_ACCOUNT: address = @0x1; //Should change this later to actual bridge address

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
        owner: address,
        token_id: u128,
        nonce: u256,
        amount: u64
    ) acquires BridgeAccount {
        let trusted_address = signer::address_of(trusted);
        assert!(trusted_address == BRIDGE_ACCOUNT, 1); // Verify trusted signer

        let bridge_account = borrow_global_mut<BridgeAccount>(trusted_address);
        let coin = coin::withdraw<AptosCoin>(trusted, amount);
        coin::deposit(owner, coin);

        let deposit = Deposit {
            owner,
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
        let coin = coin::withdraw<AptosCoin>(owner, amount);
        let bridge_account = borrow_global_mut<BridgeAccount>(BRIDGE_ACCOUNT);
        coin::deposit(BRIDGE_ACCOUNT, coin);
        let nonce = bridge_account.nonce;
        bridge_account.nonce = nonce + 1;

        let request = PendingWithdrawalRequest {
            owner: signer::address_of(owner),
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
        let _bridge_account = borrow_global_mut<BridgeAccount>(trusted_address);

        // Implement logic to confirm and close the withdrawal request
        // Remove the pending withdrawal from bridge_account
    }

    public entry fun claim_withdrawal_request(
        owner: &signer,
        token_id: u128,
        nonce: u256
    ) acquires BridgeAccount {
        let owner_address = signer::address_of(owner);
        let bridge_account = borrow_global_mut<BridgeAccount>(BRIDGE_ACCOUNT);

        // Implement logic to claim unsuccessful withdrawal request and close it
        // Remove the pending withdrawal from bridge_account and transfer coins back to owner
    }

    struct BridgeAccount has key {
        deposit_events: EventHandle<DepositEvent>,
        pending_withdrawal_events: EventHandle<PendingWithdrawalEvent>,
        nonce: u256,
    }

    fun init_module(bridge: &signer) {
        let bridge_address = signer::address_of(bridge);
        assert!(bridge_address == BRIDGE_ACCOUNT, 1); // Verify bridge signer

        move_to(bridge, BridgeAccount {
            deposit_events: account::new_event_handle<DepositEvent>(bridge),
            pending_withdrawal_events: account::new_event_handle<PendingWithdrawalEvent>(bridge),
            nonce: 0,
        });
    }
}